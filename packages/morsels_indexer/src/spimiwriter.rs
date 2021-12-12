use std::collections::BinaryHeap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::Mutex;

use rustc_hash::FxHashMap;

use crate::docinfo::BlockDocLengths;
use crate::worker::miner::DocIdAndFieldLengthsComparator;
use crate::worker::miner::TermDoc;
use crate::worker::miner::TermDocComparator;
use crate::worker::miner::WorkerBlockIndexResults;
use crate::worker::miner::WorkerMinerDocInfo;
use crate::DocInfos;
use crate::FieldInfos;
use crate::Indexer;
use crate::MainToWorkerMessage;

impl Indexer {
    pub fn write_block(
        &self,
        main_thread_block_results: WorkerBlockIndexResults,
        block_number: u32,
        is_last_block: bool,
        num_workers_writing_blocks: &mut usize,
    ) {
        // Don't block on threads that are still writing blocks (long running)
        let num_workers_to_collect = self.indexing_config.num_threads - *num_workers_writing_blocks;
        let mut worker_index_results: Vec<WorkerBlockIndexResults> = Vec::with_capacity(num_workers_to_collect + 1);
        worker_index_results.push(main_thread_block_results);

        let receive_work_barrier: Arc<Barrier> = Arc::new(Barrier::new(num_workers_to_collect));

        // Request all workers for doc miners
        for _i in 0..num_workers_to_collect {
            self.tx_main
                .send(MainToWorkerMessage::Reset(Arc::clone(&receive_work_barrier)))
                .expect("Failed to send reset message!");
        }

        if !is_last_block {
            *num_workers_writing_blocks += 1;
        }

        // Receive doc miners
        for worker_msg in self.rx_main.iter().take(num_workers_to_collect) {
            #[cfg(debug_assertions)]
            println!("Worker {} data received!", worker_msg.id);
            worker_index_results
                .push(worker_msg.block_index_results.expect("Received non doc miner message!"));
        }

        let output_folder_path = PathBuf::from(&self.output_folder_path);
        if is_last_block {
            combine_worker_results_and_write_block(
                worker_index_results,
                Arc::clone(&self.doc_infos),
                output_folder_path,
                &self.field_infos,
                block_number,
                self.start_doc_id,
                self.is_dynamic && block_number == self.start_block_number,
                self.indexing_config.num_stores_per_dir,
                self.spimi_counter,
                self.doc_id_counter,
            );
        } else {
            self.tx_main
                .send(MainToWorkerMessage::Combine {
                    worker_index_results,
                    output_folder_path,
                    block_number,
                    start_doc_id: self.start_doc_id,
                    start_block_number: self.start_block_number,
                    spimi_counter: self.spimi_counter,
                    doc_id_counter: self.doc_id_counter,
                    doc_infos: Arc::clone(&self.doc_infos),
                })
                .expect("Failed to send work message to worker!");
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn combine_worker_results_and_write_block(
    worker_index_results: Vec<WorkerBlockIndexResults>,
    doc_infos: Arc<Mutex<DocInfos>>,
    output_folder_path: PathBuf,
    field_infos: &Arc<FieldInfos>,
    block_number: u32,
    start_doc_id: u32,
    check_for_existing_field_store: bool,
    num_stores_per_dir: u32,
    spimi_counter: u32,
    doc_id_counter: u32,
) {
    let mut combined_terms: FxHashMap<String, Vec<Vec<TermDoc>>> = FxHashMap::default();

    let mut heap: BinaryHeap<DocIdAndFieldLengthsComparator> = BinaryHeap::with_capacity(worker_index_results.len());

    // Combine
    for worker_result in worker_index_results.into_iter().filter(|w| !w.doc_infos.is_empty()) {
        for (worker_term, worker_term_docs) in worker_result.terms {
            combined_terms.entry(worker_term).or_insert_with(Vec::new).push(worker_term_docs);
        }

        let mut doc_infos_iter = worker_result.doc_infos.into_iter();
        if let Some(worker_document_length) = doc_infos_iter.next() {
            heap.push(DocIdAndFieldLengthsComparator(worker_document_length, doc_infos_iter));
        }
    }

    {
        let mut sorted_doc_infos: Vec<WorkerMinerDocInfo> = Vec::with_capacity(spimi_counter as usize);

        // ---------------------------------------------
        // Heap sort by doc id
        while !heap.is_empty() {
            let mut top = heap.pop().unwrap();

            if let Some(worker_document_length) = top.1.next() {
                heap.push(DocIdAndFieldLengthsComparator(worker_document_length, top.1));
            }

            sorted_doc_infos.push(top.0);
        }
        // ---------------------------------------------

        // ---------------------------------------------
        // Store field texts
        if !sorted_doc_infos.is_empty() {
            let mut file_number = if check_for_existing_field_store {
                start_doc_id / field_infos.field_store_block_size
            } else {
                (doc_id_counter - spimi_counter) / field_infos.field_store_block_size
            };
            let mut curr_block_count = if check_for_existing_field_store {
                start_doc_id % field_infos.field_store_block_size
            } else {
                (doc_id_counter - spimi_counter) % field_infos.field_store_block_size
            };
            let mut writer = open_new_block_file(file_number, field_infos, num_stores_per_dir, block_number, check_for_existing_field_store);
            let mut is_first = true; // may mistakenly write extra comma due to dynamic indexing
    
            for worker_miner_doc_info in sorted_doc_infos.iter_mut() {
                if curr_block_count == 0 {
                    writer = open_new_block_file(file_number, field_infos, num_stores_per_dir, block_number, check_for_existing_field_store);
                } else if !is_first {
                    writer.write_all(b",").unwrap();
                }
    
                is_first = false;
    
                writer.write_all(&std::mem::take(&mut worker_miner_doc_info.field_texts)).unwrap();
    
                curr_block_count += 1;
                if curr_block_count == field_infos.field_store_block_size {
                    writer.write_all(b"]").unwrap();
                    writer.flush().unwrap();
    
                    file_number += 1;
                    curr_block_count = 0;
                }
            }
    
            if curr_block_count != 0 {
                writer.write_all(b"]").unwrap();
                writer.flush().unwrap();
            }
        } /* else {
            // possibly just a dynamic indexing run with a deletion
        } */
        // ---------------------------------------------

        // Store in global
        {
            doc_infos.lock().unwrap().all_block_doc_lengths.push(BlockDocLengths(sorted_doc_infos));
        }
    }

    {
        let mut combined_terms_vec: Vec<_> = combined_terms.into_iter().collect();
        // Sort by lexicographical order
        combined_terms_vec.sort_by(|a, b| a.0.cmp(&b.0));

        let dict_output_file_path = output_folder_path.join(format!("bsbi_block_dict_{}", block_number));
        let output_file_path = output_folder_path.join(format!("bsbi_block_{}", block_number));

        #[cfg(debug_assertions)]
        println!(
            "Writing bsbi block {} to {}, num terms {}",
            block_number,
            output_file_path.to_str().unwrap(),
            combined_terms_vec.len()
        );

        let df = File::create(dict_output_file_path)
            .expect("Failed to open temporary dictionary table for writing.");
        let mut buffered_writer_dict = BufWriter::new(df);

        let f = File::create(output_file_path).expect("Failed to open temporary dictionary string for writing.");
        let mut buffered_writer = BufWriter::with_capacity(819200, f);

        // Sort and aggregate worker docIds of each term into one vector
        for (term, workers_term_docs) in combined_terms_vec {
            buffered_writer_dict.write_all(&(term.len() as u8).to_le_bytes()).unwrap();
            buffered_writer_dict.write_all(term.as_bytes()).unwrap();
            let mut doc_freq = 0;

            // Initialise heap sort
            let mut heap: BinaryHeap<TermDocComparator> = BinaryHeap::new();
            for term_docs in workers_term_docs {
                doc_freq += term_docs.len() as u32;
                let mut iter = term_docs.into_iter();
                if let Some(term_doc) = iter.next() {
                    heap.push(TermDocComparator(term_doc, iter));
                }
            }

            buffered_writer_dict.write_all(&doc_freq.to_le_bytes()).unwrap();

            while !heap.is_empty() {
                let mut term_doc_and_iter = heap.pop().unwrap();

                buffered_writer.write_all(&term_doc_and_iter.0.doc_id.to_le_bytes()).unwrap();

                let num_fields =
                    term_doc_and_iter.0.doc_fields.iter().filter(|doc_field| doc_field.field_tf > 0).count() as u8;
                buffered_writer.write_all(&[num_fields]).unwrap();

                for (field_id, doc_field) in term_doc_and_iter.0.doc_fields.into_iter().enumerate() {
                    if doc_field.field_tf == 0 {
                        continue;
                    }

                    buffered_writer.write_all(&[field_id as u8]).unwrap();
                    buffered_writer.write_all(&doc_field.field_tf.to_le_bytes()).unwrap();

                    for pos in doc_field.positions {
                        buffered_writer.write_all(&pos.to_le_bytes()).unwrap();
                    }
                }

                if let Some(term_doc) = term_doc_and_iter.1.next() {
                    heap.push(TermDocComparator(term_doc, term_doc_and_iter.1));
                }
            }
        }

        buffered_writer.flush().unwrap();
        buffered_writer_dict.flush().unwrap();
    }
}

#[inline(always)]
fn open_new_block_file(
    file_number: u32,
    field_infos: &Arc<FieldInfos>,
    num_stores_per_dir: u32,
    block_number: u32,
    check_for_existing: bool,
) -> BufWriter<File> {
    let output_dir = field_infos.field_output_folder_path.join(
        (file_number / num_stores_per_dir).to_string()
    );
    if (file_number % num_stores_per_dir == 0)
        && !(output_dir.exists() && output_dir.is_dir())
    {
        std::fs::create_dir(&output_dir)
            .expect("Failed to create field store output dir!");
    }
    let output_file_path = output_dir.join(format!("{}--{}.json", file_number, block_number));
    if check_for_existing && output_file_path.exists() {
        // The first block for dynamic indexing might have been left halfway through somewhere before
        let mut field_store_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(output_file_path)
            .expect("Failed to open existing field store for editing");
        field_store_file
            .seek(SeekFrom::End(-1))
            .expect("Failed to seek to existing field store end");

        // Override ']' with ','
        field_store_file
            .write_all(b",")
            .expect("Failed to override existing field store ] with ,");

        BufWriter::new(field_store_file)
    } else {
        let mut writer = BufWriter::new(
            File::create(output_file_path).expect("Failed to open field store for writing."),
        );
        writer.write_all(b"[").unwrap();
        writer
    }
}
