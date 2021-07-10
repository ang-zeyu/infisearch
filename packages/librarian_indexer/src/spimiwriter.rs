use crate::Indexer;
use crate::FieldInfos;
use crate::worker::miner::WorkerMinerDocInfo;
use crate::MainToWorkerMessage;
use std::sync::Barrier;
use std::sync::Arc;
use std::sync::Mutex;
use crate::DocInfos;
use crate::worker::miner::DocIdAndFieldLengthsComparator;
use crate::worker::miner::TermDocComparator;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

use rustc_hash::FxHashMap;

use crate::worker::miner::WorkerMiner;
use crate::worker::miner::TermDoc;


impl Indexer {
    pub fn write_block (&mut self) {
        // Don't block on threads that are still writing blocks (long running)
        let mut num_workers_writing_blocks = self.num_workers_writing_blocks.lock().unwrap();
        let num_workers_to_collect = self.num_threads as usize - *num_workers_writing_blocks;
        let mut worker_miners: Vec<WorkerMiner> = Vec::with_capacity(num_workers_to_collect);

        let receive_work_barrier: Arc<Barrier> = Arc::new(Barrier::new(num_workers_to_collect));
        
        // Request all workers for doc miners
        for _i in 0..num_workers_to_collect {
            self.tx_main.send(MainToWorkerMessage::Reset(Arc::clone(&receive_work_barrier)))
                .expect("Failed to send reset message!");
        }
        
        // Receive doc miners
        for _i in 0..num_workers_to_collect {
            let worker_msg = self.rx_main.recv();
            match worker_msg {
                Ok(worker_msg_unwrapped) => {
                    println!("Received worker {} data!", worker_msg_unwrapped.id);
                    worker_miners.push(worker_msg_unwrapped.doc_miner.expect("Received non doc miner message!"));
                },
                Err(e) => panic!("Failed to receive idle message from worker! {}", e)
            }
        }

        *num_workers_writing_blocks += 1;
        self.tx_main.send(MainToWorkerMessage::Combine {
            worker_miners,
            output_folder_path: PathBuf::from(&self.output_folder_path),
            block_number: self.block_number(),
            num_docs: self.spimi_counter,
            total_num_docs: self.doc_id_counter - self.spimi_counter,
            doc_infos: Arc::clone(&self.doc_infos.as_ref().unwrap()),
        }).expect("Failed to send work message to worker!");
    
        self.spimi_counter = 0;
    }
}

pub fn combine_worker_results_and_write_block(
    worker_miners: Vec<WorkerMiner>,
    doc_infos: Arc<Mutex<DocInfos>>,
    output_folder_path: PathBuf,
    field_infos: &Arc<FieldInfos>,
    block_number: u32,
    num_docs: u32,
    total_num_docs: u32,
) {
    let spimi_block = combine_and_sort(worker_miners, doc_infos, num_docs, total_num_docs, field_infos);
    write_to_disk(spimi_block, output_folder_path, block_number);
}

fn combine_and_sort(
    worker_miners: Vec<WorkerMiner>,
    doc_infos: Arc<Mutex<DocInfos>>,
    num_docs: u32,
    total_num_docs: u32,
    field_infos: &Arc<FieldInfos>,
) -> Vec<(String, Vec<TermDoc>)> {
    let mut combined_terms: FxHashMap<String, Vec<Vec<TermDoc>>> = FxHashMap::default();

    let mut worker_lengths: Vec<Vec<WorkerMinerDocInfo>> = Vec::with_capacity(num_docs as usize);

    // Combine
    for worker_miner in worker_miners {
        for (worker_term, worker_term_docs) in worker_miner.terms {
            combined_terms
                .entry(worker_term)
                .or_insert_with(Vec::new)
                .push(worker_term_docs);
        }

        worker_lengths.push(worker_miner.doc_infos);
    }

    
    {
        let mut sorted_doc_infos: Vec<WorkerMinerDocInfo> = Vec::with_capacity(num_docs as usize);

        let mut heap: BinaryHeap<DocIdAndFieldLengthsComparator> = BinaryHeap::new();

        for idx in 0..worker_lengths.len() {
            let worker_document_lengths = worker_lengths.get_mut(idx).unwrap();
            if !worker_document_lengths.is_empty() {
                heap.push(DocIdAndFieldLengthsComparator(worker_document_lengths.pop().unwrap(), idx as usize));
            }
        }

        while !heap.is_empty() {
            let top = heap.pop().unwrap();

            let worker_document_lengths = worker_lengths.get_mut(top.1).unwrap();
            if !worker_document_lengths.is_empty() {
                heap.push(DocIdAndFieldLengthsComparator(worker_document_lengths.pop().unwrap(), top.1));
            }

            sorted_doc_infos.push(top.0);
        }
        sorted_doc_infos.reverse();

        // Store field texts
        let mut count = total_num_docs;
        let mut block_count = 0;
        let mut writer = BufWriter::new(
            File::create(field_infos.field_output_folder_path.join(format!("{}.json", count / field_infos.field_store_block_size))).unwrap()
        );
        writer.write_all(b"[").unwrap();
        for worker_miner_doc_info in sorted_doc_infos.iter_mut() {
            if block_count != 0 {
                writer.write_all(b",").unwrap();
            }
            writer.write_all(&std::mem::take(&mut worker_miner_doc_info.field_texts)).unwrap();

            block_count += 1;
            if block_count == field_infos.field_store_block_size {
                count += block_count;
                block_count = 0;
                writer.write_all(b"]").unwrap();
                writer.flush().unwrap();

                writer = BufWriter::new(
                    File::create(field_infos.field_output_folder_path.join(format!("{}.json", count / field_infos.field_store_block_size))).unwrap()
                );
                writer.write_all(b"[").unwrap();
            }
        }

        if block_count != 0 {
            writer.write_all(b"]").unwrap();
            writer.flush().unwrap();
        } else {
            writer.flush().unwrap();
            // delete
        }

        doc_infos.lock().unwrap().extend_with(sorted_doc_infos);
    }

    // Sort
    let mut sorted_entries: Vec<(String, Vec<TermDoc>)> = combined_terms.into_iter()
        .map(|mut tup| {
            // Sort and aggregate worker docIds into one vector
            let mut output: Vec<TermDoc> = Vec::new();

            let mut heap: BinaryHeap<TermDocComparator> = BinaryHeap::new();

            for i in 0..tup.1.len() {
                heap.push(TermDocComparator { val: tup.1.get_mut(i).unwrap().pop().unwrap(), idx: i });
            }

            while !heap.is_empty() {
                let top = heap.pop().unwrap();

                let worker_term_docs = tup.1.get_mut(top.idx).unwrap();
                if !worker_term_docs.is_empty() {
                    heap.push(TermDocComparator { val: worker_term_docs.pop().unwrap(), idx: top.idx });
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
    let mut buffered_writer_dict = BufWriter::new(df);

    let f = File::create(output_file_path).expect("Failed to open temporary dictionary string for writing.");
    let mut buffered_writer = BufWriter::with_capacity(819200, f);
    
    for (term, term_docs) in bsbi_block {
        // println!("Writing {}", term);

        // Write **temporary** dict table
        // Term len (1 byte) - term (term len bytes) - doc freq (4 bytes) - postings_file_offset (4 bytes)
        buffered_writer_dict.write_all(&(term.len() as u8).to_le_bytes()).unwrap();
        buffered_writer_dict.write_all(term.as_bytes()).unwrap();
        buffered_writer_dict.write_all(&(term_docs.len() as u32).to_le_bytes()).unwrap();

        // Write pl
        // doc id (4 bytes) - number of fields (1 byte)
        //   field id (1 byte) - field term frequency (4 bytes)
        //     field term position (4 bytes)
        for term_doc in term_docs.into_iter().rev() {
            buffered_writer.write_all(&term_doc.doc_id.to_le_bytes()).unwrap();

            let num_fields: u8 = term_doc.doc_fields.len() as u8;
            buffered_writer.write_all(&[num_fields]).unwrap();

            for doc_field in term_doc.doc_fields {
                buffered_writer.write_all(&[doc_field.field_id]).unwrap();
                buffered_writer.write_all(&(doc_field.field_positions.len() as u32).to_le_bytes()).unwrap();

                for pos in doc_field.field_positions {
                    buffered_writer.write_all(&pos.to_le_bytes()).unwrap();
                }
            }
        }
    }

    buffered_writer.flush().unwrap();
    buffered_writer_dict.flush().unwrap();
}
