use std::collections::HashSet;
use crate::worker::miner::DocField;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::BufWriter;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

use crate::Receiver;
use crate::utils::varint::get_var_int;
use crate::WorkerToMainMessage;
use crate::Worker;
use crate::worker::miner::TermDoc;


static POSTINGS_FILE_LIMIT: u32 = 65535;
static LAST_FIELD_MASK: u8 = 0x80; // 1000 0000

static PREFIX_FRONT_CODE: u8 = 123;
static SUBSEQUENT_FRONT_CODE: u8 = 125;

struct PostingsStream {
    idx: u32,
    buffered_dict_reader: BufReader<File>,
    buffered_reader: BufReader<File>,
    curr_term: String,
    curr_term_docs: Vec<TermDoc>,
    term_buffer: Vec<(String, Vec<TermDoc>)>,
    future_term_buffer: Vec<(String, Vec<TermDoc>)>
}

// Order by term, then block number
impl Eq for PostingsStream {}

impl Ord for PostingsStream {
    fn cmp(&self, other: &Self) -> Ordering {
        other.curr_term.cmp(&self.curr_term)
    }
}

impl PartialOrd for PostingsStream {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.curr_term.cmp(&self.curr_term))
    }
}

impl PartialEq for PostingsStream {
    fn eq(&self, other: &Self) -> bool {
        self.curr_term == other.curr_term
    }
}

impl PostingsStream {
    fn read_next(&self, num_terms: u32, worker: &Worker) {

    }

    fn get_term(&self) -> &String {
        let x = &self.curr_term;
        //self.curr_term = self.term_docs.remove(0);
        x
    }

    fn is_empty(&self) -> bool {
        self.curr_term_docs.len() == 0
    }
}


fn get_common_prefix_len(str1: &str, str2: &str) -> usize {
    let mut len = 0;

    while len < str1.len() && len < str2.len()
        && str1.chars().nth(len).unwrap() == str2.chars().nth(len).unwrap() {
        len += 1;
    }

    len
}

pub fn merge_blocks(
    num_threads: u32,
    num_blocks: u32,
    workers: &mut Vec<Worker>,
    rx_main: &Receiver<WorkerToMainMessage>,
    output_folder_path: &Path
) {
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();

    for i in 1..(num_blocks + 1) {
        let block_file_path = Path::new(output_folder_path).join(format!("bsbi_block_{}", i));
        let block_dict_file_path = Path::new(output_folder_path).join(format!("bsbi_block_dict_{}", i));

        let df = File::create(block_dict_file_path).expect("Failed to open block dictionary table for reading.");
        let f = File::create(block_file_path).expect("Failed to open block for reading.");

        let postings_stream = PostingsStream {
            idx: i - 1,
            buffered_dict_reader: BufReader::new(df),
            buffered_reader: BufReader::new(f),
            curr_term: "".to_owned(),
            curr_term_docs: Vec::new(),
            term_buffer: Vec::new(),
            future_term_buffer: Vec::new()
        };
        // postings_stream.read_next(100);

        postings_streams.push(postings_stream);
    }

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

    // N-way merge trackers
    let mut curr_term = "".to_owned();
    let mut curr_combined_term_docs: Vec<TermDoc> = Vec::new();

    // Dictionary front coding trackers
    let mut curr_common_prefix = "".to_owned();
    let mut pending_terms: Vec<String> = Vec::new();

    // Dictionary table / Postings list trackers
    let mut curr_pl = 0;
    let mut curr_pl_offset: u32 = 0;
    let move_to_next_pl = |curr_pl: &mut u32, curr_pl_offset: &mut u32, pl_writer: &mut BufWriter<File>| {
        if *curr_pl_offset > POSTINGS_FILE_LIMIT {
            *curr_pl += 1;
            *curr_pl_offset = 0;
            *pl_writer = BufWriter::new(
                File::create(
                    Path::new(output_folder_path).join(format!("pl_{}", curr_pl))
                ).expect("Failed to final dictionary string for writing.")
            );
        }
    };

    /*
     Threading algorithm:
     Whenever a postings stream's primary buffer depletes below a certain count,
     request a worker to decode more terms and postings lists into the secondary buffer.

     Once the primary buffer is fully depleted, wait for the decoding to complete if not yet done, then swap the two buffers.

     Thus, we'll need to keep track of postings streams being decoded by threads... (secondary buffers being filled)
     using a simple hashset...
     */
    let pending_postings_streams: HashSet<u32> = HashSet::new();

    while !postings_streams.is_empty() {
        let postings_stream = postings_streams.pop().unwrap();
        if postings_stream.curr_term == "" {
            continue;
        }

        // postings_stream.read_next(10);

        if curr_term == postings_stream.curr_term && !postings_streams.is_empty() {
            // Add on
            curr_combined_term_docs.extend(postings_stream.curr_term_docs);
        } else if curr_combined_term_docs.len() > 0 {
            // Commit current term's postings, dictionary table, dictionary-as-a-string

            // ---------------------------------------------
            // Dictionary table writing: gap (1 byte), doc freq (var-int), pl offset (u16)
            let difference: u8 = if curr_pl_offset == 0 { 1 } else { 0 };
            dict_table_writer.write_all(&[difference]).unwrap();
            
            dict_table_writer.write_all(&get_var_int(curr_combined_term_docs.len() as u32)).unwrap();

            dict_table_writer.write_all(&(curr_pl_offset as u16).to_le_bytes()).unwrap();

            // ---------------------------------------------
            // Postings writing
            let mut prev_doc_id = 0;
            for mut term_doc in curr_combined_term_docs {
                // Var-int compression
                let doc_id_gap_varint = get_var_int(term_doc.doc_id - prev_doc_id);
                pl_writer.write_all(&doc_id_gap_varint).unwrap();
                prev_doc_id = term_doc.doc_id;

                curr_pl_offset += (doc_id_gap_varint.len()
                    + term_doc.doc_fields.len()) as u32; // field id contribution

                let mut write_doc_field = |doc_field: DocField, pl_writer: &mut BufWriter<File>| {
                    let field_tf_varint = get_var_int(doc_field.field_tf);
                    pl_writer.write_all(&field_tf_varint).unwrap();
                    curr_pl_offset += field_tf_varint.len() as u32;

                    let mut prev_pos = 0;
                    for field_term_pos in doc_field.field_positions {
                        let pos_gap_varint = get_var_int(field_term_pos - prev_pos);
                        pl_writer.write_all(&pos_gap_varint).unwrap();
                        curr_pl_offset += pos_gap_varint.len() as u32;
                        prev_pos = field_term_pos;
                    }
                };

                let last_doc_field = term_doc.doc_fields.remove(term_doc.doc_fields.len() - 1);

                for doc_field in term_doc.doc_fields {
                    let field_id = doc_field.field_id;
                    pl_writer.write_all(&[field_id]).unwrap();
                    write_doc_field(doc_field, &mut pl_writer);
                }

                let last_field_id = last_doc_field.field_id | LAST_FIELD_MASK;
                pl_writer.write_all(&[last_field_id]).unwrap();
                write_doc_field(last_doc_field, &mut pl_writer);
            }
            // ---------------------------------------------

            // ---------------------------------------------
            // Dictionary string writing
            // With simultaneous front coding
            // For frontcoding, candidates are temporarily stored
            if pending_terms.len() == 0 {
                curr_common_prefix = curr_term.clone();
                pending_terms.push(curr_term);
                return;
            } else {
                // Compute the cost if we add this term in
                // It should be negative
                // TODO make this optimal?
                let prefix_len = get_common_prefix_len(&curr_common_prefix, &curr_term);
                let cost_from_trimming_prefix_minus_otherwise: i32 = (pending_terms.len() * (curr_common_prefix.len() - prefix_len) // num already frontcoded terms * prefix length reduction
                    + 2 // len + symbol
                    + (if pending_terms.len() == 1 { 1 } else { 0 })
                    - prefix_len) as i32;
        
                if cost_from_trimming_prefix_minus_otherwise <= 0 {
                    curr_common_prefix = curr_common_prefix[0..prefix_len].to_owned();
                    pending_terms.push(curr_term);
                } else {
                    // Write the prefix (pending_terms.len() > 1) **or** just the term pending_terms.len() == 1
                    dict_string_writer.write_all(curr_common_prefix.as_bytes()).unwrap();
                    
                    if pending_terms.len() > 1 {
                        // Write frontcoded terms...
                        dict_string_writer.write_all(&[PREFIX_FRONT_CODE]).unwrap();
                        dict_string_writer.write_all(&pending_terms.remove(0).as_bytes()[curr_common_prefix.len()..]).unwrap(); // first term suffix
            
                        for term in pending_terms {
                            dict_string_writer.write_all(&[(term.len() -  curr_common_prefix.len()) as u8]).unwrap();
                            dict_string_writer.write_all(&[SUBSEQUENT_FRONT_CODE]).unwrap();
                            dict_string_writer.write_all(&term.as_bytes()[curr_common_prefix.len()..]).unwrap();
                        }
                    }
                    pending_terms = Vec::new();
        
                    curr_common_prefix = curr_term.clone();
                    pending_terms.push(curr_term);
                }
            }
            // ---------------------------------------------

            // Reset
            move_to_next_pl(&mut curr_pl, &mut curr_pl_offset, &mut pl_writer);
            curr_term = postings_stream.curr_term;
            curr_combined_term_docs = postings_stream.curr_term_docs;
        }
    }
}