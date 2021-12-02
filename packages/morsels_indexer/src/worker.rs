pub mod miner;

use dashmap::DashMap;
use morsels_common::tokenize::Tokenizer;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::Mutex;
use std::thread;

use crossbeam::channel::{Receiver, Sender};
use crossbeam::deque::{Injector as CrossbeamInjector, Stealer as CrossbeamStealer, Worker as CrossbeamWorker};

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

pub enum IndexMsg {
    Index {
        doc_id: u32,
        loader_result: Box<dyn LoaderResult + Send>,
    },
    Stop
}

pub enum MainToWorkerMessage {
    Synchronize(Arc<Barrier>),
    Reset(Arc<Barrier>),
    Combine {
        worker_index_results: Vec<WorkerBlockIndexResults>,
        output_folder_path: PathBuf,
        block_number: u32,
        num_docs: u32,
        total_num_docs: u32,
        doc_infos: Arc<Mutex<DocInfos>>,
    },
    Index,
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

fn handle_index(local_queue: &CrossbeamWorker<IndexMsg>, doc_miner: &mut WorkerMiner, global_queue: &Arc<CrossbeamInjector<IndexMsg>>) {
    loop {
        let task = local_queue.pop().or_else(|| {
            // Go back to global queue
            std::iter::repeat_with(|| global_queue.steal_batch_and_pop(local_queue))
                .find(|s| s.is_success())
                .and_then(|s| s.success())
        }).expect("Worker should not fail in extracting index unit");

        if let IndexMsg::Index { doc_id, mut loader_result } = task {
            doc_miner.index_doc(doc_id, loader_result.get_field_texts());
        } else {
            // Push back Stop messages, if any
            while let Some(msg) = local_queue.pop() {
                global_queue.push(msg);
            }
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn worker(
    id: usize,
    sndr: Sender<WorkerToMainMessage>,
    rcvr: Receiver<MainToWorkerMessage>,
    tokenizer: Arc<dyn Tokenizer + Send + Sync>,
    field_infos: Arc<FieldInfos>,
    indexing_config: Arc<MorselsIndexingConfig>,
    expected_num_docs_per_reset: usize,
    num_workers_writing_blocks_clone: Arc<Mutex<usize>>,
    is_dynamic: bool,
    global_queue: Arc<CrossbeamInjector<IndexMsg>>,
) {
    let mut doc_miner = WorkerMiner::new(
        &field_infos,
        indexing_config.with_positions,
        expected_num_docs_per_reset,
        &tokenizer,
    );

    let local_queue: CrossbeamWorker<IndexMsg> = CrossbeamWorker::new_fifo();

    for msg in rcvr.into_iter() {
        match msg {
            MainToWorkerMessage::Index => {
                handle_index(&local_queue, &mut doc_miner, &global_queue);
            }
            MainToWorkerMessage::Combine {
                worker_index_results,
                output_folder_path,
                block_number,
                num_docs,
                total_num_docs,
                doc_infos,
            } => {
                #[cfg(debug_assertions)]
                println!("Worker {} writing spimi block {}!", id, block_number);

                spimiwriter::combine_worker_results_and_write_block(
                    worker_index_results,
                    doc_infos,
                    output_folder_path,
                    &field_infos,
                    block_number,
                    is_dynamic,
                    indexing_config.num_stores_per_dir,
                    num_docs,
                    total_num_docs,
                );

                #[cfg(debug_assertions)]
                println!("Worker {} wrote spimi block {}!", id, block_number);

                {
                    *num_workers_writing_blocks_clone.lock().unwrap() -= 1;
                }

                handle_index(&local_queue, &mut doc_miner, &global_queue);
            }
            MainToWorkerMessage::Reset(barrier) => {
                #[cfg(debug_assertions)]
                println!("Worker {} resetting!", id);

                // return the indexed documents...
                sndr.send(WorkerToMainMessage { id, block_index_results: Some(doc_miner.get_results(id)) })
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
