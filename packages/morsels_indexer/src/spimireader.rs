use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::str;
use std::io::BufWriter;
use std::io::BufReader;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use byteorder::{ByteOrder, LittleEndian};
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use smartstring::LazyCompact;
use smartstring::SmartString;

use morsels_common::bitmap;
use morsels_common::tokenize::TermInfo;
use morsels_common::utils::varint::decode_var_int;

use crate::DynamicIndexInfo;
use crate::Dictionary;
use crate::docinfo::DocInfos;
use crate::MainToWorkerMessage;
use crate::MorselsIndexingConfig;
use crate::Sender;
use crate::Receiver;
use crate::utils::varint;

static POSTINGS_STREAM_BUFFER_SIZE: u32 = 5000;
static POSTINGS_STREAM_READER_ADVANCE_READ_THRESHOLD: usize = 5000;

static POSTINGS_FILE_LIMIT: u32 = 65535;

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
    None
}

pub struct PostingsStreamReader {
    pub idx: u32,
    pub buffered_reader: BufReader<File>,
    pub buffered_dict_reader: BufReader<File>,
    pub future_term_buffer: VecDeque<TermDocsForMerge>,
    pub doc_infos_unlocked: Arc<DocInfos>,
}

impl PostingsStreamReader {
    fn read_next_batch (
        self,
        tx_main: &Sender<MainToWorkerMessage>,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
    ) {
        tx_main.send(MainToWorkerMessage::Decode {
            n: POSTINGS_STREAM_BUFFER_SIZE,
            postings_stream_reader: self,
            postings_stream_decoders,
        }).expect("Failed to request worker spimi block decode!");
    }
}

struct PostingsStream {
    idx: u32,
    is_empty: bool,
    is_reader_decoding: bool,
    curr_term: TermDocsForMerge,
    term_buffer: VecDeque<TermDocsForMerge>,
}

// Order by term, then block number
impl Eq for PostingsStream {}

impl PartialEq for PostingsStream {
    fn eq(&self, other: &Self) -> bool {
        self.curr_term.term == other.curr_term.term && self.idx == other.idx
    }
}

impl Ord for PostingsStream {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.curr_term.term.cmp(&self.curr_term.term) {
            Ordering::Equal => other.idx.cmp(&self.idx),
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        }
    }
}

impl PartialOrd for PostingsStream {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match other.curr_term.term.cmp(&self.curr_term.term) {
            Ordering::Equal => Some(other.idx.cmp(&self.idx)),
            Ordering::Greater => Some(Ordering::Greater),
            Ordering::Less => Some(Ordering::Less),
        }
    }
}

impl PostingsStream {
    // Transfer first term of term_buffer into curr_term and curr_term_docs
    fn get_term (
        &mut self,
        postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
        tx_main: &Sender<MainToWorkerMessage>,
        blocking_sndr: &Sender<()>,
        blocking_rcvr: &Receiver<()>,
        do_print_blocked_msg: bool,
    ) {
        if self.term_buffer.is_empty() {
            let mut lock = postings_stream_decoders.get_mut(&self.idx).unwrap();
            let lock_value_mut = lock.value_mut();
            match lock_value_mut {
                PostingsStreamDecoder::Reader(postings_stream_reader) => {
                    std::mem::swap(&mut postings_stream_reader.future_term_buffer, &mut self.term_buffer);
                },
                PostingsStreamDecoder::None => {
                    if do_print_blocked_msg {
                        println!("Blocked! Ouch! Consider increasing the decode buffer size...");
                    }

                    // Set to notifier
                    *lock_value_mut = PostingsStreamDecoder::Notifier(Mutex::from(blocking_sndr.clone()));

                    // Deadlock otherwise - worker will never be able to acquire postings_stream_readers_vec
                    drop(lock);

                    // Wait for worker to finish decoding...
                    blocking_rcvr.recv().unwrap();

                    // Done! Reacquire lock
                    match postings_stream_decoders.get_mut(&self.idx).unwrap().value_mut() {
                        PostingsStreamDecoder::Reader(postings_stream_reader) => {
                            std::mem::swap(&mut postings_stream_reader.future_term_buffer, &mut self.term_buffer);
                        },
                        _ => panic!("Unexpected state @get_term blocking branch")
                    }
                },
                _ => panic!("Unexpected state @get_term notifier")
            }
            self.is_reader_decoding = false;
        } else if !self.is_reader_decoding && self.term_buffer.len() < POSTINGS_STREAM_READER_ADVANCE_READ_THRESHOLD {
            // Request for an in-advance worker decode...

            match std::mem::replace(postings_stream_decoders.get_mut(&self.idx).unwrap().value_mut(), PostingsStreamDecoder::None) {
                PostingsStreamDecoder::Reader(postings_stream_reader) => {
                    postings_stream_reader.read_next_batch(tx_main, Arc::clone(postings_stream_decoders));
                    self.is_reader_decoding = true;
                },
                _ => panic!("Unexpected state @get_term")
            }
        }

        // Pluck out the first tuple
        if let Some(term_termdocs) = self.term_buffer.pop_front() {
            self.curr_term = term_termdocs;
        } else {
            self.is_empty = true;
        }
    }
}

#[inline(always)]
fn get_pl_writer(
    output_folder_path: &Path,
    curr_pl: u32,
    num_pls_per_dir: u32,
) -> BufWriter<File> {
    let dir_output_folder_path = output_folder_path.join(format!("pl_{}", curr_pl / num_pls_per_dir));
    if (curr_pl % num_pls_per_dir == 0) && !(dir_output_folder_path.exists() && dir_output_folder_path.is_dir()) {
        std::fs::create_dir(&dir_output_folder_path).expect("Failed to create pl output dir!");
    }

    BufWriter::new(
        File::create(
            dir_output_folder_path.join(Path::new(&format!("pl_{}", curr_pl)))
        ).expect("Failed to open postings list for writing.")
    )
}

fn get_common_unicode_prefix_byte_len(str1: &str, str2: &str) -> usize {
    let mut byte_len = 0;
    let mut str1_it = str1.chars();
    let mut str2_it = str2.chars();
    
    loop {
        let str1_next = str1_it.next();
        let str2_next = str2_it.next();
        if str1_next == None || str2_next == None || (str1_next.unwrap() != str2_next.unwrap()) {
            break;
        }

        byte_len += str1_next.unwrap().len_utf8();
    }

    byte_len
}

#[allow(clippy::too_many_arguments)]
fn initialise_postings_streams(
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
        let block_dict_file = File::open(block_dict_file_path).expect("Failed to open block dictionary table for reading.");

        // Transfer reader to thread and begin reads
        postings_stream_decoders.insert(idx, PostingsStreamDecoder::None);

        (PostingsStreamReader {
            idx,
            buffered_reader: BufReader::new(block_file),
            buffered_dict_reader: BufReader::new(block_dict_file),
            future_term_buffer: VecDeque::with_capacity(POSTINGS_STREAM_BUFFER_SIZE as usize),
            doc_infos_unlocked:  Arc::clone(&doc_infos),
        }).read_next_batch(tx_main, Arc::clone(&postings_stream_decoders));
    }

    // Wait for all initial decoding to finish (for the heap to have initialised)
    for idx in 1..(num_blocks + 1) {
        let mut postings_stream = PostingsStream {
            idx,
            is_empty: false,
            is_reader_decoding: true,
            curr_term: Default::default(),
            term_buffer: VecDeque::with_capacity(POSTINGS_STREAM_BUFFER_SIZE as usize), // transfer ownership of future term buffer to the main postings stream
        };
        postings_stream.get_term(&postings_stream_decoders, tx_main, blocking_sndr, blocking_rcvr, false);
        postings_streams.push(postings_stream);
    }
}

fn get_dict_writers(output_folder_path: &Path) -> (BufWriter<File>, BufWriter<File>) {
    let dict_table_writer = BufWriter::new(
        File::create(
            Path::new(output_folder_path).join("dictionaryTable")
        ).expect("Failed to open final dictionary table for writing.")
    );
    let dict_string_writer = BufWriter::new(
        File::create(
            Path::new(output_folder_path).join("dictionaryString")
        ).expect("Failed to open final dictionary string for writing.")
    );

    (dict_table_writer, dict_string_writer)
}

#[inline(always)]
fn aggregate_block_terms(
    curr_combined_term_docs: &mut Vec<TermDocsForMerge>,
    postings_streams: &mut BinaryHeap<PostingsStream>,
    postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
    tx_main: &Sender<MainToWorkerMessage>,
    blocking_sndr: &Sender<()>,
    blocking_rcvr: &Receiver<()>,
) -> (String, u32, f32) {
    curr_combined_term_docs.clear();

    let mut postings_stream = postings_streams.pop().unwrap();

    let mut doc_freq = postings_stream.curr_term.doc_freq;

    let curr_term = std::mem::take(&mut postings_stream.curr_term.term);
    let mut curr_term_max_score = postings_stream.curr_term.max_doc_term_score;
    curr_combined_term_docs.push(std::mem::take(&mut postings_stream.curr_term));

    postings_stream.get_term(&postings_stream_decoders, tx_main, blocking_sndr, blocking_rcvr, true);
    if !postings_stream.is_empty { postings_streams.push(postings_stream); }

    // Aggregate same terms from different blocks...
    while !postings_streams.is_empty() && postings_streams.peek().unwrap().curr_term.term == curr_term {
        postings_stream = postings_streams.pop().unwrap();

        doc_freq += postings_stream.curr_term.doc_freq;

        if postings_stream.curr_term.max_doc_term_score > curr_term_max_score {
            curr_term_max_score = postings_stream.curr_term.max_doc_term_score;
        }
        curr_combined_term_docs.push(std::mem::take(&mut postings_stream.curr_term));
        
        postings_stream.get_term(&postings_stream_decoders, tx_main, blocking_sndr, blocking_rcvr, true);
        if !postings_stream.is_empty { postings_streams.push(postings_stream); }
    }

    (curr_term, doc_freq, curr_term_max_score)
}

#[inline(always)]
fn write_new_term_postings(
    curr_combined_term_docs: &mut Vec<TermDocsForMerge>,
    varint_buf: &mut[u8],
    pl_writer: &mut BufWriter<File>,
    pl_offset: &mut u32,
    doc_freq: u32,
    curr_term_max_score: f32,
    num_docs: f64,
) {
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
    let max_doc_term_score: f32 = curr_term_max_score
        * (1.0 + (num_docs - doc_freq_double + 0.5) / (doc_freq_double + 0.5)).ln() as f32;
    pl_writer.write_all(&max_doc_term_score.to_le_bytes()).unwrap();
    *pl_offset += 4;
}

#[inline(always)]
fn forward_postings_list_if_needed(
    dict_table_writer: Option<&mut BufWriter<File>>,
    pl_writer: &mut BufWriter<File>,
    curr_pl: &mut u32,
    curr_pl_offset: &mut u32,
    pl_names_to_cache: &mut Vec<u32>,
    indexing_config: &MorselsIndexingConfig,
    output_folder_path: &Path,
) {
    // ---------------------------------------------
    // Split postings file if necessary
    if *curr_pl_offset > POSTINGS_FILE_LIMIT {
        // --------------------------------
        // Dictionary table writing
        // (1 byte varint = 0 in place of the docFreq varint, delimiting a new postings list)

        if let Some(dict_table_writer) = dict_table_writer {
            dict_table_writer.write_all(&[128_u8]).unwrap();
        }
        // --------------------------------

        pl_writer.flush().unwrap();

        if *curr_pl_offset > indexing_config.pl_cache_threshold {
            pl_names_to_cache.push(*curr_pl);
        }

        *curr_pl += 1;
        *curr_pl_offset = 0;
        *pl_writer = get_pl_writer(output_folder_path, *curr_pl, indexing_config.num_pls_per_dir);
    }
    // ---------------------------------------------
}

#[inline(always)]
fn frontcode_and_store_term(prev_term: &str, curr_term: &str, dict_string_writer: &mut BufWriter<File>) {
    let unicode_prefix_byte_len = get_common_unicode_prefix_byte_len(&prev_term, &curr_term);

    dict_string_writer.write_all(&[
        unicode_prefix_byte_len as u8,                      // Prefix length
        (curr_term.len() - unicode_prefix_byte_len) as u8,  // Remaining length
    ]).unwrap();
    dict_string_writer.write_all(&curr_term.as_bytes()[unicode_prefix_byte_len..]).unwrap();
}

#[allow(clippy::too_many_arguments)]
pub fn merge_blocks(
    doc_id_counter: u32,
    num_blocks: u32,
    indexing_config: &mut MorselsIndexingConfig,
    pl_names_to_cache: &mut Vec<u32>,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path,
    dynamic_index_info: &mut DynamicIndexInfo,
) {
    /*
     Gist of this function:

     Whenever a postings stream's primary buffer depletes below a certain count,
     request a worker to decode more terms and postings lists into the secondary buffer.

     Once the primary buffer is fully depleted, wait for the decoding to complete **if not yet done**, then swap the two buffers.

     We keep track of postings streams being decoded by threads... (secondary buffers being filled)
     using a concurrent HashMap (DashMap)...
     */
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();
    let postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>> = Arc::from(DashMap::with_capacity(num_blocks as usize));
    let (blocking_sndr, blocking_rcvr): (Sender<()>, Receiver<()>) = crossbeam::bounded(1);

    let num_docs_double = doc_id_counter as f64;

    // Unwrap the inner mutex to avoid locks as it is now read-only
    let doc_infos_unlocked_arc = if let Ok(doc_infos_mutex) = Arc::try_unwrap(doc_infos) {
        let mut doc_infos_unwrapped_inner = doc_infos_mutex.into_inner().unwrap();
        doc_infos_unwrapped_inner.finalize_and_flush(output_folder_path.join("docInfo"), doc_id_counter);
    
        Arc::from(doc_infos_unwrapped_inner)
    } else {
        panic!("Failed to unwrap doc info mutex from arc.");
    };

    initialise_postings_streams(
        num_blocks, output_folder_path, &mut postings_streams, &postings_stream_decoders,
        &doc_infos_unlocked_arc, tx_main, &blocking_sndr, &blocking_rcvr
    );

    /*
     N-way merge according to lexicographical order
     Sort and aggregate worker docIds into one vector
     */
    
    let (mut dict_table_writer, mut dict_string_writer) = get_dict_writers(output_folder_path);
    let mut pl_writer = get_pl_writer(output_folder_path, 0, indexing_config.num_pls_per_dir);

    // Preallocate some things
    let mut curr_combined_term_docs: Vec<TermDocsForMerge> = Vec::with_capacity(num_blocks as usize);

    // Dictionary front coding tracker
    let mut prev_term = "".to_owned();

    // Dictionary table / Postings list trackers
    let mut curr_pl = 0;
    let mut curr_pl_offset: u32 = 0;

    // Varint buffer
    let mut varint_buf: [u8; 16] = [0; 16];

    println!("Starting main decode loop...! Number of blocks {}", postings_streams.len());

    while !postings_streams.is_empty() {
        let (curr_term, doc_freq, curr_term_max_score) = aggregate_block_terms(
            &mut curr_combined_term_docs, &mut postings_streams, &postings_stream_decoders,
            tx_main, &blocking_sndr, &blocking_rcvr
        );

        // Commit the term's n-way merged postings (curr_combined_term_docs),
        // and dictionary table, dictionary-as-a-string for the term.

        // ---------------------------------------------
        // Dictionary table writing: doc freq (var-int), pl offset (u32)
        
        dict_table_writer.write_all(varint::get_var_int(doc_freq, &mut varint_buf)).unwrap();

        dict_table_writer.write_all(&curr_pl_offset.to_le_bytes()).unwrap();

        // ---------------------------------------------
        // Postings writing

        // Postings

        write_new_term_postings(
            &mut curr_combined_term_docs, &mut varint_buf, &mut pl_writer, &mut curr_pl_offset,
            doc_freq, curr_term_max_score, num_docs_double,
        );

        // ---------------------------------------------

        // ---------------------------------------------
        // Dictionary string writing

        frontcode_and_store_term(&prev_term, &curr_term, &mut dict_string_writer);

        prev_term = curr_term;

        // ---------------------------------------------

        forward_postings_list_if_needed(
            Some(&mut dict_table_writer), &mut pl_writer,
            &mut curr_pl, &mut curr_pl_offset,
            pl_names_to_cache, indexing_config, output_folder_path,
        );
    }

    dynamic_index_info.last_pl_number = if curr_pl_offset != 0 { curr_pl } else { std::cmp::min(curr_pl - 1, 0) };
    dynamic_index_info.num_docs = doc_id_counter;

    dict_table_writer.flush().unwrap();
    pl_writer.flush().unwrap();
    dict_string_writer.flush().unwrap();
}

pub fn cleanup_blocks(
    num_blocks: u32,
    output_folder_path: &Path
) {
    // Remove temporary spimi files
    for idx in 1..(num_blocks + 1) {
        let block_file_path = Path::new(output_folder_path).join(format!("bsbi_block_{}", idx));
        let block_dict_file_path = Path::new(output_folder_path).join(format!("bsbi_block_dict_{}", idx));
        std::fs::remove_file(&block_file_path).expect("Failed to cleanup temporary bsbi_block file!");
        std::fs::remove_file(&block_dict_file_path).expect("Failed to cleanup temporary bsbi_block_dict file!");
    }
}

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
        old_term_info: &Rc<TermInfo>,
        old_num_docs: f64,
        num_docs: f64,
        num_new_docs: u32,
        new_max_term_score: f32,
        curr_combined_term_docs: &mut Vec<TermDocsForMerge>,
        invalidation_vector: &[u8],
        varint_buf: &mut [u8]
    ) -> TermInfo {
        self.pl_writer.write_all(&self.pl_vec[self.pl_vec_last_offset..(old_term_info.postings_file_offset as usize)]).unwrap();

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
            let (doc_id_gap, doc_id_len) = decode_var_int(&self.pl_vec[pl_vec_pos..]);

            prev_doc_id += doc_id_gap;
            pl_vec_pos += doc_id_len;

            let start = pl_vec_pos;

            let mut is_last: u8 = 0;
            while is_last == 0 {
                is_last = self.pl_vec[pl_vec_pos] & 0x80;
                pl_vec_pos += 1;

                let (field_tf, field_tf_len) = decode_var_int(&self.pl_vec[pl_vec_pos..]);
                pl_vec_pos += field_tf_len;

                if self.with_positions {
                    for _j in 0..field_tf {
                        // Not interested in positions here, just decode and forward pos
                        pl_vec_pos += decode_var_int(&self.pl_vec[pl_vec_pos..]).1;
                    }
                }
            }

            if bitmap::check(invalidation_vector, prev_doc_id as usize) {
                new_term_info.doc_freq -= 1;
            } else {
                // Doc id gaps need to be re-encoded due to possible doc deletions
                self.pl_writer.write_all(varint::get_var_int(prev_doc_id - prev_last_valid_id, varint_buf)).unwrap();
                self.pl_writer.write_all(&self.pl_vec[start..pl_vec_pos]).unwrap();
                prev_last_valid_id = prev_doc_id;
            }
        }

        // Old max term score
        let old_doc_freq_double = old_term_info.doc_freq as f64;
        let new_doc_freq_double = new_term_info.doc_freq as f64;

        let old_max_term_score = LittleEndian::read_f32(&self.pl_vec[pl_vec_pos..])
            / (1.0 + (old_num_docs - old_doc_freq_double + 0.5) / (old_doc_freq_double + 0.5)).ln() as f32
            * (1.0 + (num_docs - new_doc_freq_double + 0.5) / (new_doc_freq_double + 0.5)).ln() as f32;
        pl_vec_pos += 4;
        
        // Add in new documents
        for term_docs in curr_combined_term_docs {
            // Link up the gap between the first doc id of the current block and the previous block
            self.pl_writer.write_all(
                varint::get_var_int(term_docs.first_doc_id - prev_last_valid_id, varint_buf)
            ).unwrap();

            prev_last_valid_id = term_docs.last_doc_id;

            self.pl_writer.write_all(&term_docs.combined_var_ints).unwrap();
        }

        // New max term score
        let new_max_term_score = new_max_term_score
            * (1.0 + (num_docs - new_doc_freq_double + 0.5) / (new_doc_freq_double + 0.5)).ln() as f32;
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

        pl_file_length_differences.insert(self.curr_pl, self.pl_writer.len() as i32 - self.pl_vec.len() as i32);

        File::create(self.output_path).unwrap().write_all(&*self.pl_writer).unwrap();
    }
}

#[inline(always)]
fn get_existing_pl_writer(
    output_folder_path: &Path,
    curr_pl: u32,
    num_pls_per_dir: u32,
    with_positions: bool,
) -> ExistingPlWriter {
    let output_path = output_folder_path
        .join(format!("pl_{}", curr_pl / num_pls_per_dir))
        .join(Path::new(&format!("pl_{}", curr_pl)));

    // Load the entire postings list into memory
    let mut pl_file = File::open(&output_path).unwrap();

    let mut pl_vec = Vec::new();
    pl_file.read_to_end(&mut pl_vec).unwrap();

    ExistingPlWriter {
        curr_pl,
        pl_vec,
        pl_writer: Vec::new(),
        pl_vec_last_offset: 0,
        with_positions,
        output_path,
    }
}

// The same as merge_blocks, but for dynamic indexing.
//
// Goes through things term-at-a-time (all terms found in the current iteration) as well,
// but is different in all other ways:
// - Updates existing postings lists of terms (add new doc ids / delete)
//   No new postings lists are created for existing terms
// - Adds new postings lists for terms that did not exist before
// - Update dictionaryTable / String info along the way,
// - But only write the dictionaryTable / dictionaryString only at the end

#[allow(clippy::too_many_arguments)]
pub fn modify_blocks(
    doc_id_counter: u32,
    num_blocks: u32,
    indexing_config: &mut MorselsIndexingConfig,
    pl_names_to_cache: &mut Vec<u32>,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path,
    dictionary: &mut Dictionary,
    dynamic_index_info: &mut DynamicIndexInfo,
) {
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();
    let postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>> = Arc::from(DashMap::with_capacity(num_blocks as usize));
    let (blocking_sndr, blocking_rcvr): (Sender<()>, Receiver<()>) = crossbeam::bounded(1);

    let old_num_docs = dynamic_index_info.num_docs as f64;
    let new_num_docs = (doc_id_counter - dynamic_index_info.num_deleted_docs) as f64;

    // Unwrap the inner mutex to avoid locks as it is now read-only
    let doc_infos_unlocked_arc = if let Ok(doc_infos_mutex) = Arc::try_unwrap(doc_infos) {
        let mut doc_infos_unwrapped_inner = doc_infos_mutex.into_inner().unwrap();
        doc_infos_unwrapped_inner.finalize_and_flush(output_folder_path.join("docInfo"), new_num_docs as u32);
    
        Arc::from(doc_infos_unwrapped_inner)
    } else {
        panic!("Failed to unwrap doc info mutex from arc.");
    };

    initialise_postings_streams(
        num_blocks, output_folder_path, &mut postings_streams, &postings_stream_decoders,
        &doc_infos_unlocked_arc, tx_main, &blocking_sndr, &blocking_rcvr
    );

    // Preallocate some things
    let mut curr_combined_term_docs: Vec<TermDocsForMerge> = Vec::with_capacity(num_blocks as usize);

    // Dictionary table / Postings list trackers
    let mut new_pl_writer = get_pl_writer(output_folder_path, dynamic_index_info.last_pl_number + 1, indexing_config.num_pls_per_dir);
    let mut new_pl = dynamic_index_info.last_pl_number + 1;
    let mut new_pls_offset: u32 = 0;
    let mut pl_file_length_differences: FxHashMap<u32, i32> = FxHashMap::default();

    let mut existing_pl_writer: Option<ExistingPlWriter> = None;
    let mut term_info_updates: FxHashMap<String, TermInfo> = FxHashMap::default();
    let mut new_term_infos: Vec<(String, TermInfo)> = Vec::new();

    let mut varint_buf: [u8; 16] = [0; 16];

    while !postings_streams.is_empty() {
        let (curr_term, doc_freq, curr_term_max_score) = aggregate_block_terms(
            &mut curr_combined_term_docs, &mut postings_streams, &postings_stream_decoders,
            tx_main, &blocking_sndr, &blocking_rcvr
        );

        let existing_term_info = dictionary.get_term_info(&curr_term);
        if let Some(old_term_info) = existing_term_info {
            // Existing term

            // Is the term_pl_writer for the same pl?
            let mut term_pl_writer = if let Some(existing_pl_writer_unwrapped) = existing_pl_writer {
                if existing_pl_writer_unwrapped.curr_pl != old_term_info.postings_file_name {
                    existing_pl_writer_unwrapped.commit(&mut pl_file_length_differences);

                    get_existing_pl_writer(
                        &output_folder_path, old_term_info.postings_file_name, indexing_config.num_pls_per_dir, indexing_config.with_positions
                    )
                } else {
                    existing_pl_writer_unwrapped
                }
            } else {
                get_existing_pl_writer(
                    &output_folder_path, old_term_info.postings_file_name, indexing_config.num_pls_per_dir, indexing_config.with_positions
                )
            };

            let new_term_info = term_pl_writer.update_term_pl(
                old_term_info,
                old_num_docs,
                new_num_docs,
                doc_freq,
                curr_term_max_score,
                &mut curr_combined_term_docs,
                &dynamic_index_info.invalidation_vector,
                &mut varint_buf,
            );

            term_info_updates.insert(curr_term, new_term_info);

            existing_pl_writer = Some(term_pl_writer);
        } else {
            // New term
            new_term_infos.push((curr_term, TermInfo {
                doc_freq,
                idf: 0.0,
                postings_file_name: new_pl,
                postings_file_offset: new_pls_offset,
            }));

            write_new_term_postings(
                &mut curr_combined_term_docs, &mut varint_buf, &mut new_pl_writer, &mut new_pls_offset,
                doc_freq, curr_term_max_score, new_num_docs,
            );
            
            forward_postings_list_if_needed(
                None, &mut new_pl_writer,
                &mut new_pl, &mut new_pls_offset,
                pl_names_to_cache, indexing_config, output_folder_path,
            );
        }
    }

    if let Some(existing_pl_writer) = existing_pl_writer {
        existing_pl_writer.commit(&mut pl_file_length_differences);
    }
    new_pl_writer.flush().unwrap();
    
    let (mut dict_table_writer, mut dict_string_writer) = get_dict_writers(output_folder_path);

    // Dictionary front coding tracker
    let mut prev_term = Rc::new(SmartString::from(""));
    let mut prev_dict_pl = 0;
    
    let mut old_pairs_sorted: Vec<_> = std::mem::take(&mut dictionary.term_infos).into_iter().collect();
    old_pairs_sorted.sort_by(|a, b| {
        match a.1.postings_file_name.cmp(&b.1.postings_file_name) {
            Ordering::Equal => {
                a.1.postings_file_offset.cmp(&b.1.postings_file_offset)
            },
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        }
    }); // Sort by old postings list order, then lexicographical
    
    // Existing terms, maybe with updates
    let mut old_pairs_before_updated: Vec<(Rc<SmartString<LazyCompact>>, Rc<TermInfo>)> = Vec::new();
    let commit_old_pairs_before_updated = |
        dict_table_writer: &mut BufWriter<File>,
        varint_buf: &mut [u8],
        old_pairs_before_updated: &mut Vec<(Rc<SmartString<LazyCompact>>, Rc<TermInfo>)>,
        curr_existing_pl_difference: i32
    | {
        for (_term, term_info) in old_pairs_before_updated.iter_mut() {
            dict_table_writer.write_all(varint::get_var_int(term_info.doc_freq, varint_buf)).unwrap();

            dict_table_writer.write_all(&((term_info.postings_file_offset as i32 + curr_existing_pl_difference) as u32).to_le_bytes()).unwrap();
        }
        old_pairs_before_updated.clear();
    };

    // Write old pairs
    // Also resolve the new postings file offsets of terms that were not touched,
    // but were in postings lists that were edited but other terms
    for (term, term_info) in old_pairs_sorted {
        frontcode_and_store_term(&prev_term, &term, &mut dict_string_writer);
        prev_term = term;

        if prev_dict_pl != term_info.postings_file_name {
            commit_old_pairs_before_updated(
                &mut dict_table_writer,
                &mut varint_buf,
                &mut old_pairs_before_updated,
                if let Some(diff) = pl_file_length_differences.get(&prev_dict_pl) { *diff } else { 0 },
            );

            prev_dict_pl = term_info.postings_file_name;
            dict_table_writer.write_all(&[128_u8]).unwrap();
        }

        if let Some(updated_term_info) = term_info_updates.get(&prev_term[..]) {
            commit_old_pairs_before_updated(
                &mut dict_table_writer,
                &mut varint_buf,
                &mut old_pairs_before_updated,
                updated_term_info.postings_file_offset as i32 - term_info.postings_file_offset as i32
            );

            dict_table_writer.write_all(varint::get_var_int(updated_term_info.doc_freq, &mut varint_buf)).unwrap();
            dict_table_writer.write_all(&(updated_term_info.postings_file_offset.to_le_bytes())).unwrap();
        } else {
            old_pairs_before_updated.push((prev_term.clone(), term_info));
        }
    }

    if !old_pairs_before_updated.is_empty() {
        commit_old_pairs_before_updated(
            &mut dict_table_writer,
            &mut varint_buf,
            &mut old_pairs_before_updated,
            if let Some(diff) = pl_file_length_differences.get(&prev_dict_pl) { *diff } else { 0 },
        );
    }

    let mut prev_term = "".to_owned();

    /*
     Attach new terms to the end
     Not ideal for frontcoding savings, but much easier and performant for incremental indexing.

     All postings lists have to be redecoded and spit out other wise.
     */
    for (term, term_info) in new_term_infos {
        if prev_dict_pl != term_info.postings_file_name {
            dict_table_writer.write_all(&[128_u8]).unwrap();
        }

        dict_table_writer.write_all(varint::get_var_int(term_info.doc_freq, &mut varint_buf)).unwrap();

        dict_table_writer.write_all(&term_info.postings_file_offset.to_le_bytes()).unwrap();
     
        
        frontcode_and_store_term(&prev_term, &term, &mut dict_string_writer);
        prev_term = term;

        prev_dict_pl = term_info.postings_file_name;
    }
    

    dict_table_writer.flush().unwrap();
    dict_string_writer.flush().unwrap();

    dynamic_index_info.last_pl_number = if new_pls_offset != 0 { new_pl } else { std::cmp::min(new_pl - 1, 0) };
    dynamic_index_info.num_docs = new_num_docs as u32;
}
