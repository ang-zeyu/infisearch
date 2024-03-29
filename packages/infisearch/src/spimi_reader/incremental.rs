use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender};
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use smartstring::LazyCompact;
use smartstring::SmartString;

use infisearch_common::{bitmap, FILE_EXT};
use infisearch_common::dictionary::TermInfo;
use infisearch_common::packed_var_int::read_bits_from;
use infisearch_common::postings_list::{
    LAST_FIELD_MASK, SHORT_FORM_MASK,
    MIN_CHUNK_SIZE, CHUNK_SIZE,
};
use infisearch_common::utils::varint::decode_var_int;

use crate::dictionary_writer::DictWriter;
use crate::field_info::FieldInfos;
use crate::incremental_info::IncrementalIndexInfo;
use crate::indexer::input_config::InfiIndexingConfig;
use crate::spimi_reader::common::PlWriter;
use crate::spimi_reader::common::{
    self, postings_stream::PostingsStream, PostingsStreamDecoder, TermDocsForMerge,
};
use crate::utils::varint;
use crate::worker::MainToWorkerMessage;

struct ExistingPlWriter {
    curr_pl: u32,
    pl_vec: Vec<u8>,
    pl_writer: Vec<u8>,
    pl_vec_last_offset: usize,
    with_positions: bool,
    output_path: PathBuf,
}

impl ExistingPlWriter {
    #[allow(clippy::too_many_arguments)]
    fn update_term_pl(
        &mut self,
        old_term_info: &TermInfo,
        num_new_docs: u32,
        curr_combined_term_docs: &mut Vec<TermDocsForMerge>,
        invalidation_vector: &[u8],
        varint_buf: &mut [u8],
    ) -> TermInfo {
        self.pl_writer
            .write_all(&self.pl_vec[self.pl_vec_last_offset..(old_term_info.postings_file_offset as usize)])
            .unwrap();

        let mut new_term_info = TermInfo {
            doc_freq: old_term_info.doc_freq + num_new_docs,
            postings_file_name: old_term_info.postings_file_name,
            postings_file_offset: self.pl_writer.len() as u32,
        };

        let mut pl_vec_pos = old_term_info.postings_file_offset as usize;
        let mut prev_last_valid_id = 0;

        let mut prev_doc_id = 0;
        for _i in 0..old_term_info.doc_freq {
            let doc_id_gap = decode_var_int(&self.pl_vec, &mut pl_vec_pos);

            prev_doc_id += doc_id_gap;

            let start = pl_vec_pos;

            let mut is_last: u8 = 0;
            while is_last == 0 {
                let next_int = self.pl_vec[pl_vec_pos];
                pl_vec_pos += 1;

                is_last = next_int & LAST_FIELD_MASK;

                let field_tf = if (next_int & SHORT_FORM_MASK) != 0 {
                    (next_int & 0b00000111) as u32
                } else {
                    decode_var_int(&self.pl_vec, &mut pl_vec_pos)
                };

                if self.with_positions {
                    // Not interested in positions here, just decode and forward pos

                    if field_tf >= MIN_CHUNK_SIZE {
                        let mut bit_pos = 0;

                        let num_chunks = (field_tf / CHUNK_SIZE)
                            + if field_tf % CHUNK_SIZE == 0 { 0 } else { 1 };
                            
                        let slice_starting_here = &self.pl_vec[pl_vec_pos..];

                        let mut read = 0;
                        for _chunk in 0..num_chunks {
                            // Read position length in this chunk
                            let chunk_len = read_bits_from(&mut bit_pos, 5, slice_starting_here) as usize;

                            for _i in 0..CHUNK_SIZE {
                                bit_pos += chunk_len;

                                read += 1;
                                if read == field_tf {
                                    break;
                                }
                            }
                        }

                        pl_vec_pos += (bit_pos / 8) + if bit_pos % 8 == 0 { 0 } else { 1 };
                    } else {
                        for _j in 0..field_tf {
                            decode_var_int(&self.pl_vec, &mut pl_vec_pos);
                        }
                    }
                }
            }

            if bitmap::check(invalidation_vector, prev_doc_id as usize) {
                new_term_info.doc_freq -= 1;
            } else {
                // Doc id gaps need to be re-encoded due to possible doc deletions
                self.pl_writer
                    .write_all(varint::get_var_int(prev_doc_id - prev_last_valid_id, varint_buf))
                    .unwrap();
                self.pl_writer.write_all(&self.pl_vec[start..pl_vec_pos]).unwrap();
                prev_last_valid_id = prev_doc_id;
            }
        }

        // Add in new documents
        for term_docs in curr_combined_term_docs {
            // Link up the gap between the first doc id of the current block and the previous block
            self.pl_writer
                .write_all(varint::get_var_int(term_docs.first_doc_id - prev_last_valid_id, varint_buf))
                .unwrap();

            prev_last_valid_id = term_docs.last_doc_id;

            self.pl_writer.write_all(&term_docs.combined_var_ints).unwrap();
        }

        self.pl_vec_last_offset = pl_vec_pos;

        new_term_info
    }

    fn commit(mut self, pl_file_length_differences: &mut FxHashMap<u32, i32>) {
        if self.pl_vec_last_offset < self.pl_vec.len() {
            self.pl_writer.write_all(&self.pl_vec[self.pl_vec_last_offset..]).unwrap();
        }

        pl_file_length_differences
            .insert(self.curr_pl, self.pl_writer.len() as i32 - self.pl_vec.len() as i32);

        File::create(self.output_path).unwrap().write_all(&*self.pl_writer).unwrap();
    }
}

// The same as merge_blocks, but for incremental indexing.
//
// Goes through things term-at-a-time (all terms found in the current iteration) as well,
// but is different in all other ways:
// - Updates existing postings lists of terms (add new doc ids / delete)
//   No new postings lists are created for existing terms
// - Adds new postings lists for terms that did not exist before
// - Update dictionary table / string info along the way,
// - But only write the dictionary table / string only at the end

#[allow(clippy::too_many_arguments)]
pub fn modify_blocks(
    has_docs_added: bool,
    num_blocks: u32,
    first_block: u32,
    last_block: u32,
    indexing_config: &InfiIndexingConfig,
    field_infos: &Arc<FieldInfos>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path,
    incremental_info: &mut IncrementalIndexInfo,
) -> DictWriter {
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();
    let postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>> =
        Arc::from(DashMap::with_capacity(num_blocks as usize));
    let (blocking_sndr, blocking_rcvr): (Sender<()>, Receiver<()>) = crossbeam::channel::bounded(1);

    if !has_docs_added {
        common::initialise_postings_stream_readers(
            first_block,
            last_block,
            output_folder_path,
            &mut postings_streams,
            &postings_stream_decoders,
            field_infos.num_scored_fields,
            tx_main,
            &blocking_sndr,
            &blocking_rcvr,
        );
    }

    let mut dict_writer = DictWriter::new();
    let mut new_pl_writer = PlWriter::new(
        output_folder_path,
        incremental_info.last_pl_number + 1,
        indexing_config.num_pls_per_dir,
    );
    
    // Preallocate some things
    let mut curr_combined_term_docs: Vec<TermDocsForMerge> = Vec::with_capacity(num_blocks as usize);

    let mut existing_pl_writers: FxHashMap<u32, ExistingPlWriter> = FxHashMap::default();
    let mut term_info_updates: FxHashMap<String, TermInfo> = FxHashMap::default();
    let mut new_term_infos: Vec<(String, TermInfo)> = Vec::new();

    let mut varint_buf: [u8; 16] = [0; 16];

    while !postings_streams.is_empty() {
        let (curr_term, doc_freq) = PostingsStream::aggregate_block_terms(
            &mut curr_combined_term_docs,
            &mut postings_streams,
            &postings_stream_decoders,
            tx_main,
            &blocking_sndr,
            &blocking_rcvr,
        );

        let existing_term_info = incremental_info.dictionary.get_term_info(&curr_term);
        if let Some(old_term_info) = existing_term_info {
            // Existing term

            let term_pl_writer = if let Some(existing) = existing_pl_writers.get_mut(&old_term_info.postings_file_name) {
                existing
            } else {
                let output_path = output_folder_path
                    .join(format!("pl_{}", old_term_info.postings_file_name / indexing_config.num_pls_per_dir))
                    .join(Path::new(&format!("pl_{}.{}", old_term_info.postings_file_name, FILE_EXT)));
            
                // Load the entire postings list into memory
                let mut pl_file = File::open(&output_path).unwrap();
            
                let mut pl_vec = Vec::new();
                pl_file.read_to_end(&mut pl_vec).unwrap();
            
                existing_pl_writers.insert(old_term_info.postings_file_name, ExistingPlWriter {
                    curr_pl: old_term_info.postings_file_name,
                    pl_vec,
                    pl_writer: Vec::with_capacity(indexing_config.pl_limit as usize),
                    pl_vec_last_offset: 0,
                    with_positions: indexing_config.with_positions,
                    output_path,
                });
                existing_pl_writers.get_mut(&old_term_info.postings_file_name).unwrap()
            };

            let new_term_info = term_pl_writer.update_term_pl(
                old_term_info,
                doc_freq,
                &mut curr_combined_term_docs,
                &incremental_info.invalidation_vector,
                &mut varint_buf,
            );

            term_info_updates.insert(curr_term, new_term_info);
        } else {
            let start_pl_offset = common::write_new_term_postings(
                &mut curr_combined_term_docs,
                &mut varint_buf,
                None,
                &mut new_pl_writer,
                &mut incremental_info.pl_names_to_cache,
                indexing_config,
                output_folder_path,
            );

            // New term
            new_term_infos.push((
                curr_term,
                TermInfo {
                    doc_freq,
                    postings_file_name: new_pl_writer.pl,
                    postings_file_offset: start_pl_offset,
                },
            ));
        }
    }

    let mut pl_file_length_differences: FxHashMap<u32, i32> = FxHashMap::default();
    for (_pl, pl_writer) in existing_pl_writers {
        pl_writer.commit(&mut pl_file_length_differences);
    }

    new_pl_writer.flush(indexing_config.pl_cache_threshold, &mut incremental_info.pl_names_to_cache);

    // ---------------------------------------------
    // Dictionary

    let mut prev_offset = 0;

    /*
     Write old terms first

     Also resolve the new postings file offsets of terms that were not touched,
     but were in postings lists that were edited by other terms.
    */
    let mut prev_term = SmartString::from("");
    let mut prev_dict_pl = 0;

    let mut old_pairs_sorted: Vec<_> = std::mem::take(&mut incremental_info.dictionary.term_infos).into_iter().collect();

    // Sort by old postings list order
    old_pairs_sorted.sort_by(|a, b| match a.1.postings_file_name.cmp(&b.1.postings_file_name) {
        Ordering::Equal => a.1.postings_file_offset.cmp(&b.1.postings_file_offset),
        Ordering::Greater => Ordering::Greater,
        Ordering::Less => Ordering::Less,
    });

    let mut term_terminfo_pairs: Vec<(SmartString<LazyCompact>, &TermInfo, (u8, u8))> = Vec::new();

    fn commit_pairs(
        dict_table_writer: &mut DictWriter,
        term_terminfo_pairs: &mut Vec<(SmartString<LazyCompact>, &TermInfo, (u8, u8))>,
        prev_offset: &mut u32,
        curr_existing_pl_difference: i32,
    ) {
        for (_term, term_info, (prefix_len, remaining_len)) in term_terminfo_pairs.iter_mut() {
            let pl_offset = (term_info.postings_file_offset as i32 + curr_existing_pl_difference) as u32;

            dict_table_writer.write_dict_table_entry(
                term_info.doc_freq,
                pl_offset, prev_offset,
                *prefix_len, *remaining_len,
            );
        }
        term_terminfo_pairs.clear();
    }

    for (term, term_info) in old_pairs_sorted {
        let prefix_and_remaining_len = dict_writer.write_term(&prev_term, &term);
        prev_term = term;

        if prev_dict_pl != term_info.postings_file_name {
            commit_pairs(
                &mut dict_writer,
                &mut term_terminfo_pairs,
                &mut prev_offset,
                if let Some(diff) = pl_file_length_differences.get(&prev_dict_pl) { *diff } else { 0 },
            );

            dict_writer.write_pl_separator();
            prev_offset = 0;
            prev_dict_pl = term_info.postings_file_name;
        }

        if let Some(updated_term_info) = term_info_updates.get(&prev_term[..]) {
            commit_pairs(
                &mut dict_writer,
                &mut term_terminfo_pairs,
                &mut prev_offset,
                updated_term_info.postings_file_offset as i32 - term_info.postings_file_offset as i32,
            );

            dict_writer.write_dict_table_entry(
                updated_term_info.doc_freq,
                updated_term_info.postings_file_offset, &mut prev_offset,
                prefix_and_remaining_len.0, prefix_and_remaining_len.1,
            );
        } else {
            term_terminfo_pairs.push((prev_term.clone(), term_info, prefix_and_remaining_len));
        }
    }

    if !term_terminfo_pairs.is_empty() {
        commit_pairs(
            &mut dict_writer,
            &mut term_terminfo_pairs,
            &mut prev_offset,
            if let Some(diff) = pl_file_length_differences.get(&prev_dict_pl) { *diff } else { 0 },
        );
    }

    /*
     Attach new terms to the end
     Not ideal for frontcoding savings, but much easier and performant for incremental indexing.

     All postings lists have to be redecoded and spit out other wise.
    */

    let mut prev_term = "".to_owned();

    for (term, term_info) in new_term_infos {
        let prefix_and_remaining_len = dict_writer.write_term(&prev_term, &term);
        prev_term = term;

        if prev_dict_pl != term_info.postings_file_name {
            dict_writer.write_pl_separator();
            prev_offset = 0;
            prev_dict_pl = term_info.postings_file_name;
        }

        dict_writer.write_dict_table_entry(
            term_info.doc_freq,
            term_info.postings_file_offset, &mut prev_offset,
            prefix_and_remaining_len.0, prefix_and_remaining_len.1,
        );
    }

    incremental_info.last_pl_number = if new_pl_writer.pl_offset != 0 || new_pl_writer.pl == 0 {
        new_pl_writer.pl
    } else {
        new_pl_writer.pl - 1
    };

    dict_writer
}
