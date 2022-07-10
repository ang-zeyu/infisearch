pub mod postings_stream;
pub mod postings_stream_reader;
pub mod terms;

use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender};
use dashmap::DashMap;

use morsels_common::FILE_EXT;
use morsels_common::dictionary::{DICTIONARY_STRING_FILE_NAME};

use self::postings_stream::{PostingsStream, POSTINGS_STREAM_BUFFER_SIZE, POSTINGS_STREAM_INITIAL_READ};
use self::postings_stream_reader::PostingsStreamReader;
use crate::indexer::input_config::MorselsIndexingConfig;
use crate::utils::bufwriter::ReusableWriter;
use crate::utils::varint;
use crate::worker::MainToWorkerMessage;

#[derive(Default)]
pub struct TermDocsForMerge {
    pub term: String,
    pub doc_freq: u32,
    pub combined_var_ints: Vec<u8>,
    pub first_doc_id: u32,
    pub last_doc_id: u32,
}

pub enum PostingsStreamDecoder {
    Reader(PostingsStreamReader),
    Notifier(Sender<()>),
    None,
}

pub struct PlWriter {
    writer: ReusableWriter,
    pl: u32,
}

impl PlWriter {
    fn change_file(&mut self, file: File, pl: u32) {
        self.writer.change_file(file);
        self.pl = pl;
    }

    pub fn flush(&mut self, pl_offset: u32, pl_cache_threshold: u32, pl_names_to_cache: &mut Vec<u32>) {
        self.writer.flush();
        if pl_offset > pl_cache_threshold {
            pl_names_to_cache.push(self.pl);
        }
    }
}

#[inline(always)]
pub fn get_pl_file(output_folder_path: &Path, curr_pl: u32, num_pls_per_dir: u32) -> File {
    let dir_output_folder_path = output_folder_path.join(format!("pl_{}", curr_pl / num_pls_per_dir));
    if (curr_pl % num_pls_per_dir == 0)
        && !(dir_output_folder_path.exists() && dir_output_folder_path.is_dir())
    {
        std::fs::create_dir(&dir_output_folder_path).expect("Failed to create pl output dir!");
    }

    File::create(dir_output_folder_path.join(Path::new(&format!("pl_{}.{}", curr_pl, FILE_EXT))))
        .expect("Failed to open postings list for writing.")
}

pub fn get_pl_writer(output_folder_path: &Path, curr_pl: u32, num_pls_per_dir: u32) -> PlWriter {
    let mut writer = ReusableWriter::new();
    writer.change_file(get_pl_file(output_folder_path, curr_pl, num_pls_per_dir));
    PlWriter { writer, pl: curr_pl }
}

pub fn get_dictstring_writer(output_folder_path: &Path) -> BufWriter<File> {
    BufWriter::new(
        File::create(Path::new(output_folder_path).join(DICTIONARY_STRING_FILE_NAME))
            .expect("Failed to open final dictionary string for writing."),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn initialise_postings_stream_readers(
    first_block: u32,
    last_block: u32,
    output_folder_path: &Path,
    postings_stream_heap: &mut BinaryHeap<PostingsStream>,
    postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
    num_scored_fields: usize,
    tx_main: &Sender<MainToWorkerMessage>,
    blocking_sndr: &Sender<()>,
    blocking_rcvr: &Receiver<()>,
) {
    // Initialize postings streams and readers, start reading
    for idx in first_block..(last_block + 1) {
        let block_file_path = Path::new(output_folder_path).join(format!("bsbi_block_{}", idx));
        let block_dict_file_path = Path::new(output_folder_path).join(format!("bsbi_block_dict_{}", idx));

        let block_file = File::open(block_file_path).expect("Failed to open block for reading.");
        let block_dict_file =
            File::open(block_dict_file_path).expect("Failed to open block dictionary table for reading.");

        // Transfer reader to thread and begin reads
        postings_stream_decoders.insert(idx, PostingsStreamDecoder::None);

        (PostingsStreamReader {
            idx,
            buffered_reader: BufReader::new(block_file),
            buffered_dict_reader: BufReader::new(block_dict_file),
            future_term_buffer: VecDeque::with_capacity(POSTINGS_STREAM_BUFFER_SIZE),
            num_scored_fields,
        })
        .read_next_batch(POSTINGS_STREAM_INITIAL_READ, tx_main, Arc::clone(postings_stream_decoders));
    }

    // Wait for all initial decoding to finish (for the heap to have initialised)
    PostingsStream::initialise_postings_streams(
        first_block,
        last_block,
        postings_stream_heap,
        postings_stream_decoders,
        tx_main,
        blocking_sndr,
        blocking_rcvr,
    );
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn write_new_term_postings(
    curr_combined_term_docs: &mut [TermDocsForMerge],
    varint_buf: &mut [u8],
    dict_table_writer: Option<&mut BufWriter<File>>,
    curr_pl: &mut u32,
    pl_writer: &mut PlWriter,
    pl_offset: &mut u32,
    prev_pl_start_offset: &mut u32,
    pl_names_to_cache: &mut Vec<u32>,
    indexing_config: &MorselsIndexingConfig,
    output_folder_path: &Path,
) -> u32 {
    // ---------------------------------------------
    // Split to new postings file if necessary

    // 16 is maximum varint size for the block_doc_id_gap_varint
    let curr_postings_max_size =
        curr_combined_term_docs.iter().fold(0, |acc, next| acc + next.combined_var_ints.len() as u32 + 16);
    let end = *pl_offset + curr_postings_max_size;

    if end > indexing_config.pl_limit {
        // --------------------------------
        // Dictionary table writing
        // (1 byte varint = 0 in place of the docFreq varint, delimiting a new postings list)

        if let Some(dict_table_writer) = dict_table_writer {
            dict_table_writer.write_all(&[128_u8]).unwrap();
        }
        // --------------------------------

        pl_writer.flush(*pl_offset, indexing_config.pl_cache_threshold, pl_names_to_cache);

        *curr_pl += 1;
        *pl_offset = 0;
        *prev_pl_start_offset = 0;

        let new_pl_file = get_pl_file(output_folder_path, *curr_pl, indexing_config.num_pls_per_dir);
        pl_writer.change_file(new_pl_file, *curr_pl);
    }
    // ---------------------------------------------

    let start_pl_offset = *pl_offset;

    let mut prev_block_last_doc_id = 0;
    for term_docs in curr_combined_term_docs.iter_mut() {
        // Link up the gap between the first doc id of the current block and the previous block
        let block_doc_id_gap_varint = varint::get_var_int(term_docs.first_doc_id - prev_block_last_doc_id, varint_buf);
        pl_writer.writer.write(block_doc_id_gap_varint);
        *pl_offset += block_doc_id_gap_varint.len() as u32;

        prev_block_last_doc_id = term_docs.last_doc_id;

        pl_writer.writer.write(&term_docs.combined_var_ints);
        *pl_offset += term_docs.combined_var_ints.len() as u32;
    }

    start_pl_offset
}

pub fn cleanup_blocks(first_block: u32, last_block: u32, output_folder_path: &Path) {
    // Remove temporary spimi files
    for idx in first_block..(last_block + 1) {
        let block_file_path = Path::new(output_folder_path).join(format!("bsbi_block_{}", idx));
        let block_dict_file_path = Path::new(output_folder_path).join(format!("bsbi_block_dict_{}", idx));
        std::fs::remove_file(&block_file_path).unwrap_or(());
        std::fs::remove_file(&block_dict_file_path).unwrap_or(());
    }
}
