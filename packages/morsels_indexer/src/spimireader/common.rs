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
use std::sync::Mutex;

use dashmap::DashMap;

use morsels_common::dictionary::{DICTIONARY_STRING_FILE_NAME, DICTIONARY_TABLE_FILE_NAME};

use crate::docinfo::DocInfos;
use crate::utils::varint;
use crate::MainToWorkerMessage;
use crate::MorselsIndexingConfig;
use crate::Receiver;
use crate::Sender;
use self::postings_stream::{PostingsStream, POSTINGS_STREAM_BUFFER_SIZE};
use self::postings_stream_reader::PostingsStreamReader;

#[derive(Default)]
pub struct TermDocsForMerge {
    pub term: String,
    pub max_doc_term_score: f32,
    pub doc_freq: u32,
    pub combined_var_ints: Vec<u8>,
    pub first_doc_id: u32,
    pub last_doc_id: u32,
}

pub enum PostingsStreamDecoder {
    Reader(PostingsStreamReader),
    Notifier(Mutex<Sender<()>>),
    None,
}

#[inline(always)]
pub fn get_pl_writer(output_folder_path: &Path, curr_pl: u32, num_pls_per_dir: u32) -> BufWriter<File> {
    let dir_output_folder_path = output_folder_path.join(format!("pl_{}", curr_pl / num_pls_per_dir));
    if (curr_pl % num_pls_per_dir == 0) && !(dir_output_folder_path.exists() && dir_output_folder_path.is_dir()) {
        std::fs::create_dir(&dir_output_folder_path).expect("Failed to create pl output dir!");
    }

    BufWriter::new(
        File::create(dir_output_folder_path.join(Path::new(&format!("pl_{}", curr_pl))))
            .expect("Failed to open postings list for writing."),
    )
}

pub fn get_dict_writers(output_folder_path: &Path) -> (BufWriter<File>, BufWriter<File>) {
    let dict_table_writer = BufWriter::new(
        File::create(Path::new(output_folder_path).join(DICTIONARY_TABLE_FILE_NAME))
            .expect("Failed to open final dictionary table for writing."),
    );
    let dict_string_writer = BufWriter::new(
        File::create(Path::new(output_folder_path).join(DICTIONARY_STRING_FILE_NAME))
            .expect("Failed to open final dictionary string for writing."),
    );

    (dict_table_writer, dict_string_writer)
}

#[allow(clippy::too_many_arguments)]
pub fn initialise_postings_streams(
    num_blocks: u32,
    output_folder_path: &Path,
    postings_streams: &mut BinaryHeap<PostingsStream>,
    postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
    doc_infos: &Arc<DocInfos>,
    tx_main: &Sender<MainToWorkerMessage>,
    blocking_sndr: &Sender<()>,
    blocking_rcvr: &Receiver<()>,
) {
    // Initialize postings streams and readers, start reading
    for idx in 1..(num_blocks + 1) {
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
            future_term_buffer: VecDeque::with_capacity(POSTINGS_STREAM_BUFFER_SIZE as usize),
            doc_infos_unlocked: Arc::clone(&doc_infos),
        })
        .read_next_batch(tx_main, Arc::clone(&postings_stream_decoders));
    }

    // Wait for all initial decoding to finish (for the heap to have initialised)
    PostingsStream::initialise_postings_streams(
        num_blocks,
        postings_streams,
        postings_stream_decoders,
        tx_main,
        blocking_sndr,
        blocking_rcvr,
    );
}


#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn write_new_term_postings(
    curr_combined_term_docs: &mut Vec<TermDocsForMerge>,
    varint_buf: &mut [u8],
    dict_table_writer: Option<&mut BufWriter<File>>,
    curr_pl: &mut u32,
    pl_writer: &mut BufWriter<File>,
    pl_offset: &mut u32,
    doc_freq: u32,
    curr_term_max_score: f32,
    num_docs: f64,
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

        pl_writer.flush().unwrap();

        if *pl_offset > indexing_config.pl_cache_threshold {
            pl_names_to_cache.push(*curr_pl);
        }

        *curr_pl += 1;
        *pl_offset = 0;
        *pl_writer = get_pl_writer(output_folder_path, *curr_pl, indexing_config.num_pls_per_dir);
    }
    // ---------------------------------------------

    let start_pl_offset = *pl_offset;

    let mut prev_block_last_doc_id = 0;
    for term_docs in curr_combined_term_docs.iter_mut() {
        // Link up the gap between the first doc id of the current block and the previous block
        let block_doc_id_gap_varint = varint::get_var_int(term_docs.first_doc_id - prev_block_last_doc_id, varint_buf);
        pl_writer.write_all(block_doc_id_gap_varint).unwrap();
        *pl_offset += block_doc_id_gap_varint.len() as u32;

        prev_block_last_doc_id = term_docs.last_doc_id;

        pl_writer.write_all(&term_docs.combined_var_ints).unwrap();
        *pl_offset += term_docs.combined_var_ints.len() as u32;
    }

    let doc_freq_double = doc_freq as f64;
    let max_doc_term_score: f32 =
        curr_term_max_score * (1.0 + (num_docs - doc_freq_double + 0.5) / (doc_freq_double + 0.5)).ln() as f32;
    pl_writer.write_all(&max_doc_term_score.to_le_bytes()).unwrap();
    *pl_offset += 4;

    start_pl_offset
}

pub fn cleanup_blocks(num_blocks: u32, output_folder_path: &Path) {
    // Remove temporary spimi files
    for idx in 1..(num_blocks + 1) {
        let block_file_path = Path::new(output_folder_path).join(format!("bsbi_block_{}", idx));
        let block_dict_file_path = Path::new(output_folder_path).join(format!("bsbi_block_dict_{}", idx));
        std::fs::remove_file(&block_file_path).expect("Failed to cleanup temporary bsbi_block file!");
        std::fs::remove_file(&block_dict_file_path).expect("Failed to cleanup temporary bsbi_block_dict file!");
    }
}
