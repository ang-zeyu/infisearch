use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::BufWriter;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use byteorder::{ByteOrder, LittleEndian};
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use smartstring::LazyCompact;
use smartstring::SmartString;

use morsels_common::bitmap;
use morsels_common::tokenize::TermInfo;
use morsels_common::utils::idf::get_idf;
use morsels_common::utils::varint::decode_var_int;
use morsels_common::DOC_INFO_FILE_NAME;

use crate::docinfo::DocInfos;
use crate::spimireader::common::{
    self, postings_stream::PostingsStream, terms, PostingsStreamDecoder, TermDocsForMerge,
};
use crate::utils::varint;
use crate::IncrementalIndexInfo;
use crate::MainToWorkerMessage;
use crate::MorselsIndexingConfig;
use crate::Receiver;
use crate::Sender;

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
        old_num_docs: f64,
        num_docs: f64,
        num_new_docs: u32,
        new_max_term_score: f32,
        curr_combined_term_docs: &mut Vec<TermDocsForMerge>,
        invalidation_vector: &[u8],
        varint_buf: &mut [u8],
    ) -> TermInfo {
        self.pl_writer
            .write_all(&self.pl_vec[self.pl_vec_last_offset..(old_term_info.postings_file_offset as usize)])
            .unwrap();

        let mut new_term_info = TermInfo {
            doc_freq: old_term_info.doc_freq + num_new_docs,
            idf: 0.0, // unused
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
                is_last = self.pl_vec[pl_vec_pos] & 0x80;
                pl_vec_pos += 1;

                let field_tf = decode_var_int(&self.pl_vec, &mut pl_vec_pos);

                if self.with_positions {
                    for _j in 0..field_tf {
                        // Not interested in positions here, just decode and forward pos
                        decode_var_int(&self.pl_vec, &mut pl_vec_pos);
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

        // Old max term score
        let old_doc_freq_double = old_term_info.doc_freq as f64;
        let new_doc_freq_double = new_term_info.doc_freq as f64;

        let old_max_term_score = LittleEndian::read_f32(&self.pl_vec[pl_vec_pos..])
            / (get_idf(old_num_docs, old_doc_freq_double)
            * get_idf(num_docs, new_doc_freq_double)) as f32;
        pl_vec_pos += 4;

        // Add in new documents
        for term_docs in curr_combined_term_docs {
            // Link up the gap between the first doc id of the current block and the previous block
            self.pl_writer
                .write_all(varint::get_var_int(term_docs.first_doc_id - prev_last_valid_id, varint_buf))
                .unwrap();

            prev_last_valid_id = term_docs.last_doc_id;

            self.pl_writer.write_all(&term_docs.combined_var_ints).unwrap();
        }

        // New max term score
        let new_max_term_score = new_max_term_score * get_idf(num_docs, new_doc_freq_double) as f32;
        if new_max_term_score > old_max_term_score {
            self.pl_writer.write_all(&new_max_term_score.to_le_bytes()).unwrap();
        } else {
            self.pl_writer.write_all(&old_max_term_score.to_le_bytes()).unwrap();
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
    is_deletion_only_run: bool,
    doc_id_counter: u32,
    num_blocks: u32,
    first_block: u32,
    last_block: u32,
    indexing_config: &MorselsIndexingConfig,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path,
    incremental_info: &mut IncrementalIndexInfo,
) {
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();
    let postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>> =
        Arc::from(DashMap::with_capacity(num_blocks as usize));
    let (blocking_sndr, blocking_rcvr): (Sender<()>, Receiver<()>) = crossbeam::channel::bounded(1);

    let old_num_docs = incremental_info.num_docs as f64;
    let new_num_docs = (doc_id_counter - incremental_info.num_deleted_docs) as f64;

    // Unwrap the inner mutex to avoid locks as it is now read-only
    let doc_infos_unlocked_arc = {
        let mut doc_infos_unwrapped_inner = Arc::try_unwrap(doc_infos)
            .expect("No thread should be holding doc infos arc when merging blocks")
            .into_inner()
            .expect("No thread should be holding doc infos mutex when merging blocks");
        doc_infos_unwrapped_inner
            .finalize_and_flush(output_folder_path.join(DOC_INFO_FILE_NAME), new_num_docs as u32);

        Arc::from(doc_infos_unwrapped_inner)
    };

    if !is_deletion_only_run {
        common::initialise_postings_stream_readers(
            first_block,
            last_block,
            output_folder_path,
            &mut postings_streams,
            &postings_stream_decoders,
            &doc_infos_unlocked_arc,
            tx_main,
            &blocking_sndr,
            &blocking_rcvr,
        );
    }

    // Preallocate some things
    let mut curr_combined_term_docs: Vec<TermDocsForMerge> = Vec::with_capacity(num_blocks as usize);

    // Dictionary table / Postings list trackers
    let mut new_pl_writer = common::get_pl_writer(
        output_folder_path,
        incremental_info.last_pl_number + 1,
        indexing_config.num_pls_per_dir,
    );
    let mut new_pl = incremental_info.last_pl_number + 1;
    let mut new_pls_offset: u32 = 0;

    let mut existing_pl_writers: FxHashMap<u32, ExistingPlWriter> = FxHashMap::default();
    let mut term_info_updates: FxHashMap<String, TermInfo> = FxHashMap::default();
    let mut new_term_infos: Vec<(String, TermInfo)> = Vec::new();

    let mut varint_buf: [u8; 16] = [0; 16];

    while !postings_streams.is_empty() {
        let (curr_term, doc_freq, curr_term_max_score) = PostingsStream::aggregate_block_terms(
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
                    .join(Path::new(&format!("pl_{}", old_term_info.postings_file_name)));
            
                // Load the entire postings list into memory
                let mut pl_file = File::open(&output_path).unwrap();
            
                let mut pl_vec = Vec::new();
                pl_file.read_to_end(&mut pl_vec).unwrap();
            
                existing_pl_writers.insert(old_term_info.postings_file_name, ExistingPlWriter {
                    curr_pl: old_term_info.postings_file_name,
                    pl_vec,
                    pl_writer: Vec::new(),
                    pl_vec_last_offset: 0,
                    with_positions: indexing_config.with_positions,
                    output_path,
                });
                existing_pl_writers.get_mut(&old_term_info.postings_file_name).unwrap()
            };

            let new_term_info = term_pl_writer.update_term_pl(
                old_term_info,
                old_num_docs,
                new_num_docs,
                doc_freq,
                curr_term_max_score,
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
                &mut new_pl,
                &mut new_pl_writer,
                &mut new_pls_offset,
                &mut 0,
                doc_freq,
                curr_term_max_score,
                new_num_docs,
                &mut incremental_info.pl_names_to_cache,
                indexing_config,
                output_folder_path,
            );

            // New term
            new_term_infos.push((
                curr_term,
                TermInfo {
                    doc_freq,
                    idf: 0.0,
                    postings_file_name: new_pl,
                    postings_file_offset: start_pl_offset,
                },
            ));
        }
    }

    let mut pl_file_length_differences: FxHashMap<u32, i32> = FxHashMap::default();
    for (_pl, pl_writer) in existing_pl_writers {
        pl_writer.commit(&mut pl_file_length_differences);
    }

    new_pl_writer.flush(new_pls_offset, indexing_config.pl_cache_threshold, &mut incremental_info.pl_names_to_cache);

    // ---------------------------------------------
    // Dictionary

    let (mut dict_table_writer, mut dict_string_writer) = common::get_dict_writers(output_folder_path);
    let mut prev_offset = 0;

    /*
     Write old terms first

     Also resolve the new postings file offsets of terms that were not touched,
     but were in postings lists that were edited by other terms.
    */
    let mut prev_term = Rc::new(SmartString::from(""));
    let mut prev_dict_pl = 0;

    let mut old_pairs_sorted: Vec<_> = std::mem::take(&mut incremental_info.dictionary.term_infos).into_iter().collect();

    // Sort by old postings list order
    old_pairs_sorted.sort_by(|a, b| match a.1.postings_file_name.cmp(&b.1.postings_file_name) {
        Ordering::Equal => a.1.postings_file_offset.cmp(&b.1.postings_file_offset),
        Ordering::Greater => Ordering::Greater,
        Ordering::Less => Ordering::Less,
    });

    let mut term_terminfo_pairs: Vec<(Rc<SmartString<LazyCompact>>, TermInfo)> = Vec::new();

    fn commit_pairs(
        dict_table_writer: &mut BufWriter<File>,
        varint_buf: &mut [u8],
        term_terminfo_pairs: &mut Vec<(Rc<SmartString<LazyCompact>>, TermInfo)>,
        prev_offset: &mut u32,
        curr_existing_pl_difference: i32,
    ) {
        for (_term, term_info) in term_terminfo_pairs.iter_mut() {
            dict_table_writer.write_all(varint::get_var_int(term_info.doc_freq, varint_buf)).unwrap();

            let pl_offset = (term_info.postings_file_offset as i32 + curr_existing_pl_difference) as u32;

            dict_table_writer.write_all(varint::get_var_int(pl_offset - *prev_offset, varint_buf)).unwrap();

            *prev_offset = pl_offset;
        }
        term_terminfo_pairs.clear();
    }

    for (term, term_info) in old_pairs_sorted {
        terms::frontcode_and_store_term(&prev_term, &term, &mut dict_string_writer);
        prev_term = term;

        if prev_dict_pl != term_info.postings_file_name {
            commit_pairs(
                &mut dict_table_writer,
                &mut varint_buf,
                &mut term_terminfo_pairs,
                &mut prev_offset,
                if let Some(diff) = pl_file_length_differences.get(&prev_dict_pl) { *diff } else { 0 },
            );

            dict_table_writer.write_all(&[128_u8]).unwrap();
            prev_offset = 0;
            prev_dict_pl = term_info.postings_file_name;
        }

        if let Some(updated_term_info) = term_info_updates.get(&prev_term[..]) {
            commit_pairs(
                &mut dict_table_writer,
                &mut varint_buf,
                &mut term_terminfo_pairs,
                &mut prev_offset,
                updated_term_info.postings_file_offset as i32 - term_info.postings_file_offset as i32,
            );

            dict_table_writer
                .write_all(varint::get_var_int(updated_term_info.doc_freq, &mut varint_buf))
                .unwrap();
            dict_table_writer
                .write_all(varint::get_var_int(
                    updated_term_info.postings_file_offset - prev_offset,
                    &mut varint_buf,
                ))
                .unwrap();
            prev_offset = updated_term_info.postings_file_offset;
        } else {
            term_terminfo_pairs.push((prev_term.clone(), term_info));
        }
    }

    if !term_terminfo_pairs.is_empty() {
        commit_pairs(
            &mut dict_table_writer,
            &mut varint_buf,
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
        terms::frontcode_and_store_term(&prev_term, &term, &mut dict_string_writer);
        prev_term = term;

        if prev_dict_pl != term_info.postings_file_name {
            dict_table_writer.write_all(&[128_u8]).unwrap();
            prev_offset = 0;
            prev_dict_pl = term_info.postings_file_name;
        }

        dict_table_writer.write_all(varint::get_var_int(term_info.doc_freq, &mut varint_buf)).unwrap();
        dict_table_writer
            .write_all(varint::get_var_int(term_info.postings_file_offset - prev_offset, &mut varint_buf))
            .unwrap();

        prev_offset = term_info.postings_file_offset;
    }

    dict_table_writer.flush().unwrap();
    dict_string_writer.flush().unwrap();

    incremental_info.last_pl_number = if new_pls_offset != 0 || new_pl == 0 {
        new_pl
    } else {
        new_pl - 1
    };
}
