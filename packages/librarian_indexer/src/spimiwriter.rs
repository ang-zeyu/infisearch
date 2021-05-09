use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

use crate::Receiver;
use crate::WorkerMiner;
use crate::WorkerToMainMessage;
use crate::Worker;
use crate::worker::miner::TermDoc;
use crate::worker::miner::TermDocComparator;

fn combine_and_sort(worker_miners: Vec<WorkerMiner>) -> Vec<(String, Vec<TermDoc>)> {
    let mut combined_terms: HashMap<String, Vec<Vec<TermDoc>>> = HashMap::new();

    // Combine
    for worker_miner in worker_miners {
        for (worker_term, worker_term_docs) in worker_miner.terms {
            combined_terms
                .entry(worker_term)
                .or_insert(Vec::new())
                .push(worker_term_docs);
        }
    }

    // Sort
    let mut sorted_entries: Vec<(String, Vec<TermDoc>)> = combined_terms.into_iter()
        .map(|mut tup| {
            // Sort and aggregate worker docIds into one vector
            let mut output: Vec<TermDoc> = Vec::new();

            let mut heap: BinaryHeap<TermDocComparator> = BinaryHeap::new();
            let mut next_indices: Vec<u32> = Vec::new();

            for i in 0..tup.1.len() {
                heap.push(TermDocComparator { val: tup.1.get_mut(i).unwrap().remove(0), idx: i });
                next_indices.push(1)
            }

            while !heap.is_empty() {
                let top = heap.pop().unwrap();

                let worker_term_docs = tup.1.get_mut(top.idx).unwrap();
                if !worker_term_docs.is_empty() {
                    heap.push(TermDocComparator { val: worker_term_docs.remove(0), idx: top.idx });
                }

                output.push(top.val);
            }
            
            (tup.0, output)
        }).collect();

    // Sort terms by lexicographical order
    sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));

    sorted_entries
}

fn write_to_disk(bsbi_block: Vec<(String, Vec<TermDoc>)>, output_folder_path: &Path, bsbi_block_number: u32) {
    let dict_output_file_path = Path::new(output_folder_path).join(format!("bsbi_block_dict_{}", bsbi_block_number));
    let output_file_path = Path::new(output_folder_path).join(format!("bsbi_block_{}", bsbi_block_number));

    println!("Writing bsbi block {} to {}, num terms {}", bsbi_block_number, output_file_path.to_str().unwrap(), bsbi_block.len());

    let df = File::create(dict_output_file_path).expect("Failed to open temporary dictionary table for writing.");
    let mut buffered_writer_dict = BufWriter::new(df);

    let f = File::create(output_file_path).expect("Failed to open temporary dictionary string for writing.");
    let mut buffered_writer = BufWriter::new(f);
    
    let mut postings_file_offset: u32 = 0;
    for (term, term_docs) in bsbi_block {
        // println!("Writing {}", term);

        let term_byte_len: u32 = term.len().try_into().unwrap();
        buffered_writer_dict.write_all(&term_byte_len.to_le_bytes()).unwrap();
        buffered_writer_dict.write_all(term.as_bytes()).unwrap();
        buffered_writer_dict.write_all(&(term_docs.len() as u32).to_le_bytes()).unwrap();
        buffered_writer_dict.write_all(&postings_file_offset.to_le_bytes()).unwrap();

        // buffered_writer.write_all(&term_id.to_le_bytes()).unwrap();
        for term_doc in term_docs {
            buffered_writer.write_all(&term_doc.doc_id.to_le_bytes()).unwrap();

            let num_fields: u8 = term_doc.doc_fields.len().try_into().unwrap();
            buffered_writer.write_all(&[num_fields]).unwrap();

            postings_file_offset += 5;
            for doc_field in term_doc.doc_fields {
                buffered_writer.write_all(&[doc_field.field_id]).unwrap();
                buffered_writer.write_all(&doc_field.field_tf.to_le_bytes()).unwrap();

                postings_file_offset += 5;
                for pos in doc_field.field_positions {
                    buffered_writer.write_all(&pos.to_le_bytes()).unwrap();
                    postings_file_offset += 4;
                }
            }
        }
    }

    buffered_writer.flush().unwrap();
    buffered_writer_dict.flush().unwrap();
}

pub fn write_block (
    num_threads: u32,
    spimi_counter: &mut u32,
    block_number: u32,
    workers: &mut Vec<Worker>,
    rx_main: &Receiver<WorkerToMainMessage>, 
    output_folder_path: &Path
) {
    // SPIMI logic

    let mut worker_miners: Vec<WorkerMiner> = Vec::new();

    // Receive idle messages
    for i in 0..num_threads {
        let worker_msg = rx_main.recv();
        match worker_msg {
            Ok(worker_msg_unwrapped) => {
                if let Some(doc_miner_unwrapped) = worker_msg_unwrapped.doc_miner {
                    panic!("Failed to receive idle message from worker!");
                } else {
                    println!("Worker {} idle message received", worker_msg_unwrapped.id);
                }
            },
            Err(e) => panic!("Failed to receive idle message from worker! {}", e)
        }
    }

    // Request doc miner move
    for worker in workers {
        println!("Requesting doc miner move! {}", worker.id);
        worker.receive_work();
    }

    // Receive doc miners
    for _i in 0..num_threads {
        let worker_msg = rx_main.recv();
        match worker_msg {
            Ok(worker_msg_unwrapped) => {
                if let Some(doc_miner_unwrapped) = worker_msg_unwrapped.doc_miner {
                    println!("Received worker {} data!", worker_msg_unwrapped.id);
                    worker_miners.push(doc_miner_unwrapped);
                } else {
                    panic!("Unexpected message received from worker {}!", worker_msg_unwrapped.id);
                }
            },
            Err(e) => panic!("Failed to receive message from worker! {}", e)
        }
    }

    // Aggregate the lists into the block, and sort it according to term
    let spimi_block = combine_and_sort(worker_miners);

    // Write the block
    write_to_disk(spimi_block, output_folder_path, block_number);
    println!("Wrote spimi block {}", block_number);

    *spimi_counter = 0;
}
