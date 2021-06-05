use crate::MainToWorkerMessage;
use crate::docinfo::DocInfos;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::str;
use std::io::BufWriter;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

use crate::Sender;
use crate::Receiver;
use crate::utils::varint;

static POSTINGS_STREAM_BUFFER_SIZE: u32 = 5000;
static POSTINGS_STREAM_READER_ADVANCE_READ_THRESHOLD: usize = 5000;

static POSTINGS_FILE_LIMIT: u32 = 65535;
static LAST_FIELD_MASK: u8 = 0x80; // 1000 0000

static PREFIX_FRONT_CODE: u8 = 123;     // {
static SUBSEQUENT_FRONT_CODE: u8 = 125; // }

pub struct DocFieldForMerge {
    pub field_id: u8,
    pub field_tf_and_positions_varint: Vec<u8>,
}

pub struct TermDocForMerge {
    pub doc_id: u32,
    pub doc_fields: Vec<DocFieldForMerge>
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
    pub future_term_buffer: VecDeque<(String, Vec<TermDocForMerge>, f32)>,
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
    curr_term: String,
    curr_term_max_score: f32,
    curr_term_docs: Vec<TermDocForMerge>,
    term_buffer: VecDeque<(String, Vec<TermDocForMerge>, f32)>,
}

// Order by term, then block number
impl Eq for PostingsStream {}

impl PartialEq for PostingsStream {
    fn eq(&self, other: &Self) -> bool {
        self.curr_term == other.curr_term && self.idx == other.idx
    }
}

impl Ord for PostingsStream {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.curr_term.cmp(&self.curr_term) {
            Ordering::Equal => other.idx.cmp(&self.idx),
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        }
    }
}

impl PartialOrd for PostingsStream {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match other.curr_term.cmp(&self.curr_term) {
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
        if let Some(term_termdocs_triple) = self.term_buffer.pop_front() {
            self.curr_term = term_termdocs_triple.0;
            self.curr_term_docs = term_termdocs_triple.1;
            self.curr_term_max_score = term_termdocs_triple.2;
        } else {
            self.is_empty = true;
        }
    }
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

pub fn merge_blocks(
    doc_id_counter: u32,
    num_blocks: u32,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path
) {
    /*
     Threading algorithm:
     Whenever a postings stream's primary buffer depletes below a certain count,
     request a worker to decode more terms and postings lists into the secondary buffer.

     Once the primary buffer is fully depleted, wait for the decoding to complete if not yet done, then swap the two buffers.

     Thus, we'll need to keep track of postings streams being decoded by threads... (secondary buffers being filled)
     using a simple hashset...
     */
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();
    let postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>> = Arc::from(DashMap::with_capacity(num_blocks as usize));
    let (blocking_sndr, blocking_rcvr): (Sender<()>, Receiver<()>) = crossbeam::bounded(1);

    // let (tx_stream, rx_stream) : (Sender<WorkerToMainMessage>, Receiver<WorkerToMainMessage>) = std::sync::mpsc::channel();

    // Unwrap the inner mutex to avoid locks as it is now read-only
    let doc_infos_unlocked_arc = if let Ok(doc_infos_mutex) = Arc::try_unwrap(doc_infos) {
        let mut doc_infos_unwrapped_inner = doc_infos_mutex.into_inner().unwrap();
        doc_infos_unwrapped_inner.divide_field_lengths();
        doc_infos_unwrapped_inner.flush(output_folder_path.join("docInfo"));
    
        Arc::from(doc_infos_unwrapped_inner)
    } else {
        panic!("Failed to unwrap doc info mutex from arc.");
    };

    let doc_id_counter_float = doc_id_counter as f64;

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
            buffered_reader: BufReader::with_capacity(819200, block_file),
            buffered_dict_reader: BufReader::with_capacity(819200, block_dict_file),
            future_term_buffer: VecDeque::with_capacity(POSTINGS_STREAM_BUFFER_SIZE as usize),
            doc_infos_unlocked:  Arc::clone(&doc_infos_unlocked_arc),
        }).read_next_batch(tx_main, Arc::clone(&postings_stream_decoders));
    }

    // Wait for all initial decoding to finish...
    for idx in 1..(num_blocks + 1) {
        let mut postings_stream = PostingsStream {
            idx,
            is_empty: false,
            is_reader_decoding: true,
            curr_term: "".to_owned(),
            curr_term_max_score: 0.0,
            curr_term_docs: Vec::new(),
            term_buffer: VecDeque::with_capacity(POSTINGS_STREAM_BUFFER_SIZE as usize), // transfer ownership of future term buffer to the main postings stream
        };
        postings_stream.get_term(&postings_stream_decoders, tx_main, &blocking_sndr, &blocking_rcvr, false);
        postings_streams.push(postings_stream);
    }
    println!("Initialized postings streams...");

    // N-way merge according to lexicographical order
    // Sort and aggregate worker docIds into one vector
    
    let mut dict_table_writer = BufWriter::new(
        File::create(
            Path::new(output_folder_path).join("dictionaryTable")
        ).expect("Failed to open final dictionary table for writing.")
    );
    let mut dict_string_writer = BufWriter::new(
        File::create(
            Path::new(output_folder_path).join("dictionaryString")
        ).expect("Failed to final dictionary string for writing.")
    );
    let mut pl_writer = BufWriter::new(
        File::create(
            Path::new(output_folder_path).join("pl_0")
        ).expect("Failed to final dictionary string for writing.")
    );

    // Preallocate some things
    let mut curr_combined_term_docs: Vec<Vec<TermDocForMerge>> = Vec::with_capacity(num_blocks as usize);

    // Dictionary front coding trackers
    let mut prev_common_prefix = "".to_owned();
    let mut pending_terms: Vec<String> = Vec::new();
    let write_pending_terms = |dict_string_writer: &mut BufWriter<File>, prev_common_prefix: &mut String, pending_terms: &mut Vec<String>| {
        let curr_term = pending_terms.remove(0);

        // Write the first term's full length
        dict_string_writer.write_all(&[curr_term.len() as u8]).unwrap();
        // Write the prefix (if there are frontcoded terms) **or** just the term (pending_terms.len() == 1)
        dict_string_writer.write_all(prev_common_prefix.as_bytes()).unwrap();
                
        if !pending_terms.is_empty() {
            // Write frontcoded terms...
            dict_string_writer.write_all(&[PREFIX_FRONT_CODE]).unwrap();
            dict_string_writer.write_all(&curr_term.as_bytes()[prev_common_prefix.len()..]).unwrap(); // first term suffix

            for term in pending_terms {
                dict_string_writer.write_all(&[(term.len() -  prev_common_prefix.len()) as u8]).unwrap();
                dict_string_writer.write_all(&[SUBSEQUENT_FRONT_CODE]).unwrap();
                dict_string_writer.write_all(&term.as_bytes()[prev_common_prefix.len()..]).unwrap();
            }
        }
    };

    // Dictionary table / Postings list trackers
    let mut curr_pl = 0;
    let mut curr_pl_offset: u32 = 0;

    // Varint buffer
    let mut varint_buf: [u8; 16] = [0; 16];

    println!("Starting main decode loop...! Number of blocks {}", postings_streams.len());

    while !postings_streams.is_empty() {
        curr_combined_term_docs.clear();

        let mut postings_stream = postings_streams.pop().unwrap();
        // println!("term {} idx {} first doc {}", postings_stream.curr_term, postings_stream.idx, postings_stream.curr_term_docs[0].doc_id);
        
        let mut doc_freq = postings_stream.curr_term_docs.len() as u32;

        let curr_term = std::mem::take(&mut postings_stream.curr_term);
        let mut curr_term_max_score = postings_stream.curr_term_max_score;
        curr_combined_term_docs.push(std::mem::take(&mut postings_stream.curr_term_docs));

        postings_stream.get_term(&postings_stream_decoders, tx_main, &blocking_sndr, &blocking_rcvr, true);
        if !postings_stream.is_empty { postings_streams.push(postings_stream); }

        // Aggregate same terms from different blocks...
        while !postings_streams.is_empty() && postings_streams.peek().unwrap().curr_term == curr_term {
            postings_stream = postings_streams.pop().unwrap();

            doc_freq += postings_stream.curr_term_docs.len() as u32;

            if postings_stream.curr_term_max_score > curr_term_max_score {
                curr_term_max_score = postings_stream.curr_term_max_score;
            }
            curr_combined_term_docs.push(std::mem::take(&mut postings_stream.curr_term_docs));
            
            postings_stream.get_term(&postings_stream_decoders, tx_main, &blocking_sndr, &blocking_rcvr, true);
            if !postings_stream.is_empty { postings_streams.push(postings_stream); }
        }

        // Commit the term's n-way merged postings (curr_combined_term_docs),
        // and dictionary table, dictionary-as-a-string for the term.

        // ---------------------------------------------
        // Dictionary table writing: doc freq (var-int), pl offset (u16)
        
        dict_table_writer.write_all(varint::get_var_int(doc_freq, &mut varint_buf)).unwrap();

        dict_table_writer.write_all(&(curr_pl_offset as u16).to_le_bytes()).unwrap();

        // ---------------------------------------------
        // Postings writing
        // And doc norms length calculation

        let mut prev_doc_id = 0;
        for term_docs in curr_combined_term_docs.iter_mut() {
            for term_doc in term_docs {
                // println!("term {} curr {} prev {}", curr_term, term_doc.doc_id, prev_doc_id);
                let doc_id_gap_varint = varint::get_var_int(term_doc.doc_id - prev_doc_id, &mut varint_buf);
                pl_writer.write_all(doc_id_gap_varint).unwrap();
                prev_doc_id = term_doc.doc_id;
    
                curr_pl_offset += (doc_id_gap_varint.len()
                    + term_doc.doc_fields.len()) as u32; // field id contribution
    
                let mut write_doc_field = |doc_field: &DocFieldForMerge, pl_writer: &mut BufWriter<File>| {
                    pl_writer.write_all(&doc_field.field_tf_and_positions_varint).unwrap();
                    curr_pl_offset += doc_field.field_tf_and_positions_varint.len() as u32;
                };
    
                let last_doc_field = term_doc.doc_fields.remove(term_doc.doc_fields.len() - 1);
    
                for doc_field in term_doc.doc_fields.iter_mut() {
                    pl_writer.write_all(&[doc_field.field_id]).unwrap();
                    write_doc_field(doc_field, &mut pl_writer);
                }
    
                pl_writer.write_all(&[last_doc_field.field_id | LAST_FIELD_MASK]).unwrap();
                write_doc_field(&last_doc_field, &mut pl_writer);
            }
        }

        // ---------------------------------------------

        // ---------------------------------------------
        // Dictionary table writing: max term score for any document (f32)

        let doc_freq_double = doc_freq as f64;
        let max_doc_term_score: f32 = curr_term_max_score
            * (1.0 + (doc_id_counter_float - doc_freq_double + 0.5) / (doc_freq_double + 0.5)).ln() as f32;
        dict_table_writer.write_all(&max_doc_term_score.to_le_bytes()).unwrap();

        // ---------------------------------------------
        // Dictionary string writing
        // With simultaneous front coding
        // For frontcoding, candidates are temporarily stored
        if pending_terms.is_empty() {
            prev_common_prefix = curr_term.clone();
            pending_terms.push(curr_term);
        } else {
            // Compute the cost if we add this term in, it should be <= 0 to also frontcode this term
            // TODO make this optimal?
            let unicode_prefix_byte_len = get_common_unicode_prefix_byte_len(&prev_common_prefix, &curr_term);
            // println!("{} {} ", prev_common_prefix.len(), unicode_prefix_byte_len);

            // How much bytes do we add / lose by frontcoding this term?
            let frontcode_cost: i32 = if pending_terms.len() == 1 {
                    2   // 2 symbols - PREFIX_FRONT_CODE and SUBSEQUENT_FRONT_CODE
                    + 2 // the bar for the first match should be higher
                    - unicode_prefix_byte_len as i32 /* expands to + (prev_term.len() - unicode_prefix_byte_len) - prev_term.len() */
            } else {
                (pending_terms.len() * (prev_common_prefix.len() - unicode_prefix_byte_len)) as i32 // num already frontcoded terms * prefix length reduction
                    + 1 // 1 symbol
                    - unicode_prefix_byte_len as i32 /* expands to + (prev_term.len() - unicode_prefix_byte_len) - prev_term.len() */
            };
    
            if frontcode_cost < 0 {
                prev_common_prefix = prev_common_prefix[0..unicode_prefix_byte_len].to_owned();
                pending_terms.push(curr_term);
            } else {
                write_pending_terms(&mut dict_string_writer, &mut prev_common_prefix, &mut pending_terms);

                prev_common_prefix = curr_term.clone();
                pending_terms = vec![curr_term];
            }
        }
        // ---------------------------------------------

        // ---------------------------------------------
        // Split postings file if necessary
        if curr_pl_offset > POSTINGS_FILE_LIMIT {
            // --------------------------------
            // Dictionary table writing
            // (1 byte varint = 0 in place of the docFreq varint, delimiting a new postings list)

            dict_table_writer.write_all(&[128_u8]).unwrap();
            // --------------------------------

            pl_writer.flush().unwrap();

            curr_pl += 1;
            curr_pl_offset = 0;
            pl_writer = BufWriter::new(
                File::create(
                    Path::new(output_folder_path).join(format!("pl_{}", curr_pl))
                ).expect("Failed to create new buffered writer for new postings list.")
            );
        }
        // ---------------------------------------------
    }

    println!("Commiting pending terms");

    // Commit frontcoded terms
    write_pending_terms(&mut dict_string_writer, &mut prev_common_prefix, &mut pending_terms);

    dict_table_writer.flush().unwrap();
    pl_writer.flush().unwrap();
    dict_string_writer.flush().unwrap();
}