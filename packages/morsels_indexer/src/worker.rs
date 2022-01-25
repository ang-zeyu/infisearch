pub mod miner;

use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::Mutex;
use std::thread;

use crossbeam::channel::{Receiver, Sender};

use morsels_common::tokenize::IndexerTokenizer;

use crate::i_debug;
use crate::loader::LoaderResult;
use crate::spimireader::common::{postings_stream_reader::PostingsStreamReader, PostingsStreamDecoder};
use crate::spimiwriter;
use crate::worker::miner::WorkerBlockIndexResults;
use crate::DocInfos;
use crate::FieldInfos;
use crate::Indexer;
use crate::MorselsIndexingConfig;
use miner::WorkerMiner;

pub struct Worker {
    pub id: usize,
    pub join_handle: thread::JoinHandle<()>,
}

impl Indexer {
    pub fn terminate_all_workers(self) {
        drop(self.tx_main);

        for worker in self.workers {
            worker.join_handle.join().expect("Failed to join worker.");
        }
    }

    pub fn wait_on_all_workers(&self) {
        let receive_work_barrier: Arc<Barrier> = Arc::new(Barrier::new(self.indexing_config.num_threads + 1));
        for _i in 0..self.indexing_config.num_threads {
            self.tx_main.send(MainToWorkerMessage::Synchronize(Arc::clone(&receive_work_barrier))).unwrap();
        }
        receive_work_barrier.wait();
    }
}

pub enum MainToWorkerMessage {
    Synchronize(Arc<Barrier>),
    Reset(Arc<Barrier>),
    Combine {
        worker_index_results: Vec<WorkerBlockIndexResults>,
        output_folder_path: PathBuf,
        block_number: u32,
        start_doc_id: u32,
        check_for_existing_field_store: bool,
        spimi_counter: u32,
        doc_id_counter: u32,
        doc_infos: Arc<Mutex<DocInfos>>,
    },
    Index {
        doc_id: u32,
        loader_result: Box<dyn LoaderResult + Send>,
    },
    Decode {
        n: usize,
        postings_stream_reader: PostingsStreamReader,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
    },
}

pub struct WorkerToMainMessage {
    pub id: usize,
    pub block_index_results: Option<WorkerBlockIndexResults>,
}

#[allow(clippy::too_many_arguments)]
pub fn worker(
    id: usize,
    sndr: Sender<WorkerToMainMessage>,
    rcvr: Receiver<MainToWorkerMessage>,
    tokenizer: Arc<dyn IndexerTokenizer + Send + Sync>,
    field_infos: Arc<FieldInfos>,
    indexing_config: Arc<MorselsIndexingConfig>,
    expected_num_docs_per_reset: usize,
    num_workers_writing_blocks_clone: Arc<Mutex<usize>>,
) {
    let mut doc_miner = WorkerMiner::new(
        &field_infos,
        indexing_config.with_positions,
        expected_num_docs_per_reset,
        &tokenizer,
        #[cfg(debug_assertions)]
        id,
    );

    for msg in rcvr.into_iter() {
        match msg {
            MainToWorkerMessage::Index { doc_id, mut loader_result } => {
                doc_miner.index_doc(doc_id, loader_result.get_field_texts());
            }
            MainToWorkerMessage::Combine {
                worker_index_results,
                output_folder_path,
                block_number,
                start_doc_id,
                check_for_existing_field_store,
                spimi_counter,
                doc_id_counter,
                doc_infos,
            } => {
                sndr.send(WorkerToMainMessage { id: 0, block_index_results: None }).expect("Worker failed to notify combine started");

                i_debug!("Worker {} writing spimi block {}!", id, block_number);

                spimiwriter::combine_worker_results_and_write_block(
                    worker_index_results,
                    doc_infos,
                    output_folder_path,
                    &field_infos,
                    block_number,
                    start_doc_id,
                    check_for_existing_field_store,
                    indexing_config.num_docs_per_block,
                    spimi_counter,
                    doc_id_counter,
                );

                i_debug!("Worker {} wrote spimi block {}!", id, block_number);

                {
                    *num_workers_writing_blocks_clone.lock().unwrap() -= 1;
                }

                i_debug!("Worker {} decremented num_workers_writing_blocks_clone!", id);
            }
            MainToWorkerMessage::Reset(barrier) => {
                i_debug!("Worker {} resetting!", id);

                // return the indexed documents...
                sndr.send(WorkerToMainMessage { id, block_index_results: Some(doc_miner.get_results()) })
                    .expect("Failed to send message back to main thread!");

                barrier.wait();
            }
            MainToWorkerMessage::Synchronize(barrier) => {
                barrier.wait();
            }
            MainToWorkerMessage::Decode { n, postings_stream_reader, postings_stream_decoders } => {
                postings_stream_reader.decode_next_n(
                    n,
                    postings_stream_decoders,
                    indexing_config.with_positions,
                    &field_infos,
                );
            }
        }
    }
}
