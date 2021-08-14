use std::sync::Barrier;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use rustc_hash::FxHashMap;

use crate::docinfo::BlockDocLengths;
use crate::DocInfos;
use crate::FieldInfos;
use crate::Indexer;
use crate::MainToWorkerMessage;
use crate::WorkerToMainMessage;
use crate::worker::miner::DocIdAndFieldLengthsComparator;
use crate::worker::miner::TermDoc;
use crate::worker::miner::TermDocComparator;
use crate::worker::WorkerBlockIndexResults;
use crate::worker::miner::WorkerMinerDocInfo;


impl Indexer {
    #[allow(clippy::too_many_arguments)]
    pub fn write_block(
        num_workers_writing_blocks: &Arc<Mutex<usize>>,
        num_threads: usize,
        tx_main: &mut crossbeam::Sender<MainToWorkerMessage>,
        rx_main: &mut crossbeam::Receiver<WorkerToMainMessage>,
        output_folder_path: PathBuf,
        block_number: u32,
        spimi_counter: u32,
        total_num_docs: u32, //self.doc_id_counter - self.spimi_counter
        doc_infos: &Option<Arc<Mutex<DocInfos>>>,
    ) {
        // Don't block on threads that are still writing blocks (long running)
        let mut num_workers_writing_blocks = num_workers_writing_blocks.lock().unwrap();
        let num_workers_to_collect = num_threads - *num_workers_writing_blocks;
        let mut worker_index_results: Vec<WorkerBlockIndexResults> = Vec::with_capacity(num_workers_to_collect);

        let receive_work_barrier: Arc<Barrier> = Arc::new(Barrier::new(num_workers_to_collect));
        
        // Request all workers for doc miners
        for _i in 0..num_workers_to_collect {
            tx_main.send(MainToWorkerMessage::Reset(Arc::clone(&receive_work_barrier)))
                .expect("Failed to send reset message!");
        }
        
        // Receive doc miners
        for _i in 0..num_workers_to_collect {
            let worker_msg = rx_main.recv();
            match worker_msg {
                Ok(worker_msg_unwrapped) => {
                    println!("Worker {} data received!", worker_msg_unwrapped.id);
                    worker_index_results.push(worker_msg_unwrapped.block_index_results.expect("Received non doc miner message!"));
                },
                Err(e) => panic!("Failed to receive idle message from worker! {}", e)
            }
        }

        *num_workers_writing_blocks += 1;
        tx_main.send(MainToWorkerMessage::Combine {
            worker_index_results,
            output_folder_path,
            block_number,
            num_docs: spimi_counter,
            total_num_docs,
            doc_infos: Arc::clone(doc_infos.as_ref().unwrap()),
        }).expect("Failed to send work message to worker!");
    }
}

#[inline(always)]
fn get_field_store_writer(
    field_output_folder_path: &Path,
    count: u32,
    field_store_block_size: u32,
    num_stores_per_dir: u32,
) -> BufWriter<File> {
    let store_num = count / field_store_block_size;
    let dir_output_folder_path = field_output_folder_path.join((store_num / num_stores_per_dir).to_string());
    if (store_num % num_stores_per_dir == 0) && !(dir_output_folder_path.exists() && dir_output_folder_path.is_dir()) {
        std::fs::create_dir(&dir_output_folder_path).expect("Failed to create field store output dir!");
    }

    BufWriter::new(
        File::create(
            dir_output_folder_path.join(format!("{}.json", store_num))
        ).expect("Failed to open field store for writing.")
    )
}

#[allow(clippy::too_many_arguments)]
pub fn combine_worker_results_and_write_block(
    worker_index_results: Vec<WorkerBlockIndexResults>,
    doc_infos: Arc<Mutex<DocInfos>>,
    output_folder_path: PathBuf,
    field_infos: &Arc<FieldInfos>,
    block_number: u32,
    num_stores_per_dir: u32,
    num_docs: u32,
    total_num_docs: u32,
) {
    let mut combined_terms: FxHashMap<String, Vec<Vec<TermDoc>>> = FxHashMap::default();

    let mut heap: BinaryHeap<DocIdAndFieldLengthsComparator> = BinaryHeap::with_capacity(worker_index_results.len());

    // Combine
    for worker_result in worker_index_results {
        for (worker_term, worker_term_docs) in worker_result.terms {
            combined_terms
                .entry(worker_term)
                .or_insert_with(Vec::new)
                .push(worker_term_docs);
        }

        let mut doc_infos_iter = worker_result.doc_infos.into_iter();
        if let Some(worker_document_length) = doc_infos_iter.next() {
            heap.push(DocIdAndFieldLengthsComparator(worker_document_length, doc_infos_iter));
        }
    }
    
    {
        let mut sorted_doc_infos: Vec<WorkerMinerDocInfo> = Vec::with_capacity(num_docs as usize);

        // Heap sort by doc id
        while !heap.is_empty() {
            let mut top = heap.pop().unwrap();

            if let Some(worker_document_length) = top.1.next() {
                heap.push(DocIdAndFieldLengthsComparator(worker_document_length, top.1));
            }

            sorted_doc_infos.push(top.0);
        }

        // Store field texts
        let mut count = total_num_docs;
        let mut block_count = 0;
        let mut writer = BufWriter::new(File::create(field_infos.field_output_folder_path.join("nul")).unwrap());
        for worker_miner_doc_info in sorted_doc_infos.iter_mut() {
            block_count += 1;

            if block_count == 1 {
                writer = get_field_store_writer(
                    &field_infos.field_output_folder_path,
                    count,
                    field_infos.field_store_block_size,
                    num_stores_per_dir,
                );
                writer.write_all(b"[").unwrap();
            } else {
                writer.write_all(b",").unwrap();
            }

            writer.write_all(&std::mem::take(&mut worker_miner_doc_info.field_texts)).unwrap();

            if block_count == field_infos.field_store_block_size {
                writer.write_all(b"]").unwrap();
                writer.flush().unwrap();

                count += block_count;
                block_count = 0;
            }
        }

        if block_count != 0 {
            writer.write_all(b"]").unwrap();
            writer.flush().unwrap();
        }

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

        println!("Writing bsbi block {} to {}, num terms {}", block_number, output_file_path.to_str().unwrap(), combined_terms_vec.len());

        let df = File::create(dict_output_file_path).expect("Failed to open temporary dictionary table for writing.");
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

                let num_fields = term_doc_and_iter.0.doc_fields
                    .iter()
                    .filter(|doc_field| doc_field.field_tf > 0)
                    .count() as u8;
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
