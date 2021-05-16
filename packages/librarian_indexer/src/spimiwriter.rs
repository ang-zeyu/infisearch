use crate::worker::miner::TermDocComparator;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::path::Path;

use crate::Receiver;
use crate::WorkerMiner;
use crate::WorkerToMainMessage;
use crate::Worker;
use crate::worker::miner::TermDoc;

pub fn write_block (
    num_threads: u32,
    spimi_counter: &mut u32,
    block_number: u32,
    workers: &[Worker],
    rx_main: &Receiver<WorkerToMainMessage>, 
    output_folder_path: &Path
) {
    // SPIMI logic
    let mut worker_miners: Vec<WorkerMiner> = Vec::with_capacity(num_threads as usize);
    
    // Receive available messages, request workers for doc miners
    // num_threads availability messages and num_threads doc_miner messages should be received in total
    for _i in 0..(2 * num_threads) {
        let worker_msg = rx_main.recv();
        match worker_msg {
            Ok(worker_msg_unwrapped) => {
                if let Some(doc_miner_unwrapped) = worker_msg_unwrapped.doc_miner {
                    println!("Received worker {} data!", worker_msg_unwrapped.id);
                    worker_miners.push(doc_miner_unwrapped);
                } else {
                    println!("Requesting doc miner move for worker {}!", worker_msg_unwrapped.id);
                    workers[worker_msg_unwrapped.id].receive_work();
                }
            },
            Err(e) => panic!("Failed to receive idle message from worker! {}", e)
        }
    }

    // wait here to avoid receiving messages from the same workers above more than once
    Worker::make_all_workers_available(&workers);

    let combine_and_sort_worker = Worker::get_available_worker(workers, rx_main);
    combine_and_sort_worker.combine_and_sort_block(worker_miners, PathBuf::new().join(output_folder_path), block_number);

    *spimi_counter = 0;
}

pub fn combine_worker_results_and_write_block(
    worker_miners: Vec<WorkerMiner>,
    output_folder_path: PathBuf,
    block_number: u32
) {
    let spimi_block = combine_and_sort(worker_miners);
    write_to_disk(spimi_block, output_folder_path, block_number);
}

fn combine_and_sort(worker_miners: Vec<WorkerMiner>) -> Vec<(String, Vec<TermDoc>)> {
    let mut combined_terms: HashMap<String, Vec<Vec<TermDoc>>> = HashMap::new();

    // Combine
    for worker_miner in worker_miners {
        for (worker_term, worker_term_docs) in worker_miner.terms {
            combined_terms
                .entry(worker_term)
                .or_insert_with(Vec::new)
                .push(worker_term_docs);
        }
    }

    // Sort
    let mut sorted_entries: Vec<(String, Vec<TermDoc>)> = combined_terms.into_iter()
        .map(|mut tup| {
            // Sort and aggregate worker docIds into one vector
            let mut output: Vec<TermDoc> = Vec::new();

            let mut heap: BinaryHeap<TermDocComparator> = BinaryHeap::new();

            for i in 0..tup.1.len() {
                heap.push(TermDocComparator { val: tup.1.get_mut(i).unwrap().remove(0), idx: i });
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

fn write_to_disk(bsbi_block: Vec<(String, Vec<TermDoc>)>, output_folder_path: PathBuf, bsbi_block_number: u32) {
    let dict_output_file_path = output_folder_path.join(format!("bsbi_block_dict_{}", bsbi_block_number));
    let output_file_path = output_folder_path.join(format!("bsbi_block_{}", bsbi_block_number));

    println!("Writing bsbi block {} to {}, num terms {}", bsbi_block_number, output_file_path.to_str().unwrap(), bsbi_block.len());

    let df = File::create(dict_output_file_path).expect("Failed to open temporary dictionary table for writing.");
    let mut buffered_writer_dict = BufWriter::with_capacity(819200, df);

    let f = File::create(output_file_path).expect("Failed to open temporary dictionary string for writing.");
    let mut buffered_writer = BufWriter::with_capacity(819200, f);
    
    let mut postings_file_offset: u32 = 0;
    for (term, term_docs) in bsbi_block {
        // println!("Writing {}", term);

        // Write **temporary** dict table
        // Term len (4 bytes) - term (term len bytes) - doc freq (4 bytes) - postings_file_offset (4 bytes)
        buffered_writer_dict.write_all(&(term.len() as u32).to_le_bytes()).unwrap();
        buffered_writer_dict.write_all(term.as_bytes()).unwrap();
        buffered_writer_dict.write_all(&(term_docs.len() as u32).to_le_bytes()).unwrap();
        buffered_writer_dict.write_all(&postings_file_offset.to_le_bytes()).unwrap();

        // Write pl
        for term_doc in term_docs {
            buffered_writer.write_all(&term_doc.doc_id.to_le_bytes()).unwrap();

            let num_fields: u8 = term_doc.doc_fields.len() as u8;
            buffered_writer.write_all(&[num_fields]).unwrap();

            postings_file_offset += 5;
            for doc_field in term_doc.doc_fields {
                buffered_writer.write_all(&[doc_field.field_id]).unwrap();
                buffered_writer.write_all(&(doc_field.field_positions.len() as u32).to_le_bytes()).unwrap();

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
