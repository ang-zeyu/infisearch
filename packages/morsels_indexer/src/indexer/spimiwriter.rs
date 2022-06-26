
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

use rustc_hash::FxHashMap;

use crate::{i_debug, spimiwriter};
use crate::worker::MainToWorkerMessage;
use crate::worker::miner::WorkerBlockIndexResults;
use super::Indexer;

impl Indexer {
    pub fn merge_block(
        &self,
        mut main_thread_block_results: WorkerBlockIndexResults,
        block_number: u32,
        is_last_block: bool,
    ) -> Vec<FxHashMap<u32, Vec<String>>> {
        // Don't block on threads that are still writing blocks (long running)
        let mut num_workers_writing_blocks = self.num_workers_writing_blocks
            .lock()
            .expect("Main thread failed to acquire num_workers_writing_blocks lock");
        let num_workers_to_collect = self.indexing_config.num_threads - *num_workers_writing_blocks;
        let mut worker_index_results: Vec<WorkerBlockIndexResults> = Vec::with_capacity(num_workers_to_collect + 1);
        let mut secondary_inv_mappings = Vec::with_capacity(num_workers_to_collect + 1);
        secondary_inv_mappings.push(std::mem::take(&mut main_thread_block_results.secondary_inv_mappings));
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
            i_debug!("Worker {} data received!", worker_msg.id);

            let mut worker_block_index_result = worker_msg
                .block_index_results
                .expect("Received non doc miner message!");

            secondary_inv_mappings.push(
                std::mem::take(&mut worker_block_index_result.secondary_inv_mappings),
            );

            worker_index_results.push(worker_block_index_result);
        }

        drop(num_workers_writing_blocks);

        let output_folder_path = PathBuf::from(&self.output_folder_path);
        let check_for_existing_field_store = self.is_incremental && block_number == self.start_block_number;
        if is_last_block {
            spimiwriter::combine_worker_results_and_write_block(
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

        secondary_inv_mappings
    }
}
