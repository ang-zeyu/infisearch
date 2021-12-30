use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::Mutex;

use rustc_hash::FxHashMap;

use crate::docinfo::BlockDocLengths;
use crate::worker::miner::DocIdAndFieldLengthsComparator;
use crate::worker::miner::TermDoc;
use crate::worker::miner::WorkerBlockIndexResults;
use crate::worker::miner::WorkerMinerDocInfo;
use crate::DocInfos;
use crate::FieldInfos;
use crate::Indexer;
use crate::MainToWorkerMessage;

mod fields;
mod write_block;

impl Indexer {
    pub fn merge_block(
        &self,
        main_thread_block_results: WorkerBlockIndexResults,
        block_number: u32,
        is_last_block: bool,
    ) {
        // Don't block on threads that are still writing blocks (long running)
        let mut num_workers_writing_blocks = self.num_workers_writing_blocks
            .lock()
            .expect("Main thread failed to acquire num_workers_writing_blocks lock");
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

        *num_workers_writing_blocks += 1;

        // Receive doc miners
        for worker_msg in self.rx_main.iter().take(num_workers_to_collect) {
            #[cfg(debug_assertions)]
            println!("Worker {} data received!", worker_msg.id);
            worker_index_results
                .push(worker_msg.block_index_results.expect("Received non doc miner message!"));
        }

        drop(num_workers_writing_blocks);

        let output_folder_path = PathBuf::from(&self.output_folder_path);
        let check_for_existing_field_store = self.is_incremental && block_number == self.start_block_number;
        if is_last_block {
            combine_worker_results_and_write_block(
                worker_index_results,
                Arc::clone(&self.doc_infos),
                output_folder_path,
                &self.field_infos,
                block_number,
                self.start_doc_id,
                check_for_existing_field_store,
                self.indexing_config.num_docs_per_block,
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
                    check_for_existing_field_store,
                    spimi_counter: self.spimi_counter,
                    doc_id_counter: self.doc_id_counter,
                    doc_infos: Arc::clone(&self.doc_infos),
                })
                .expect("Failed to send work message to worker!");
            if self.rx_main.recv().expect("Main failed to receive msg after combine msg sent").block_index_results.is_some() {
                panic!("Main received unexpected msg after combine msg sent")
            }
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
    num_docs_per_block: u32,
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
            fields::store_fields(
                check_for_existing_field_store,
                start_doc_id,
                field_infos,
                doc_id_counter,
                spimi_counter,
                num_docs_per_block,
                block_number,
                &mut sorted_doc_infos
            );

            #[cfg(debug_assertions)]
            println!("Num docs in block {}: {}", block_number, sorted_doc_infos.len());
        } else {
            // possibly just a incremental indexing run with a deletion
            #[cfg(debug_assertions)]
            println!("Encountered empty block {}", block_number);
        }
        // ---------------------------------------------

        if !sorted_doc_infos.is_empty() {
            doc_infos.lock().unwrap().all_block_doc_lengths.push(BlockDocLengths(sorted_doc_infos));
        }
    }

    write_block::write_block(combined_terms, output_folder_path, block_number);
}


