pub mod miner;

use dashmap::DashMap;
use morsels_common::tokenize::Tokenizer;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::Mutex;
use std::thread;

use crossbeam::Receiver;
use crossbeam::Sender;

use crate::loader::LoaderResult;
use crate::spimireader::PostingsStreamDecoder;
use crate::spimireader::PostingsStreamReader;
use crate::spimiwriter;
use crate::worker::miner::WorkerBlockIndexResults;
use crate::DocInfos;
use crate::FieldInfos;
use miner::WorkerMiner;

pub struct Worker {
    pub id: usize,
    pub join_handle: thread::JoinHandle<()>,
}

impl Worker {
    pub fn terminate_all_workers(workers: Vec<Worker>, tx_main: Sender<MainToWorkerMessage>) {
        for _worker in &workers {
            tx_main.send(MainToWorkerMessage::Terminate).expect("Failed to request worker termination!");
        }

        for worker in workers {
            worker.join_handle.join().expect("Failed to join worker.");
        }
    }

    pub fn wait_on_all_workers(tx_main: &Sender<MainToWorkerMessage>, num_threads: usize) {
        let receive_work_barrier: Arc<Barrier> = Arc::new(Barrier::new(num_threads + 1));
        for _i in 0..num_threads {
            tx_main.send(MainToWorkerMessage::Synchronize(Arc::clone(&receive_work_barrier))).unwrap();
        }
        receive_work_barrier.wait();
    }
}

pub enum MainToWorkerMessage {
    Synchronize(Arc<Barrier>),
    Reset(Arc<Barrier>),
    Terminate,
    Combine {
        worker_index_results: Vec<WorkerBlockIndexResults>,
        output_folder_path: PathBuf,
        block_number: u32,
        num_docs: u32,
        total_num_docs: u32,
        doc_infos: Arc<Mutex<DocInfos>>,
    },
    Index {
        doc_id: u32,
        loader_result: Box<dyn LoaderResult + Send>,
    },
    Decode {
        n: u32,
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
    tokenizer: Arc<dyn Tokenizer + Send + Sync>,
    field_infos: Arc<FieldInfos>,
    num_stores_per_dir: u32,
    with_positions: bool,
    expected_num_docs_per_reset: usize,
    num_workers_writing_blocks_clone: Arc<Mutex<usize>>,
    is_dynamic: bool,
) {
    let mut doc_miner = WorkerMiner::new(&field_infos, with_positions, expected_num_docs_per_reset, &tokenizer);

    loop {
        let msg = rcvr.recv().expect("Failed to receive message on worker side!");
        match msg {
            MainToWorkerMessage::Index { doc_id, mut loader_result } => {
                doc_miner.index_doc(doc_id, loader_result.get_field_texts());
            }
            MainToWorkerMessage::Combine {
                worker_index_results,
                output_folder_path,
                block_number,
                num_docs,
                total_num_docs,
                doc_infos,
            } => {
                spimiwriter::combine_worker_results_and_write_block(
                    worker_index_results,
                    doc_infos,
                    output_folder_path,
                    &field_infos,
                    block_number,
                    is_dynamic,
                    num_stores_per_dir,
                    num_docs,
                    total_num_docs,
                );

                #[cfg(debug_assertions)]
                println!("Worker {} wrote spimi block {}!", id, block_number);

                {
                    *num_workers_writing_blocks_clone.lock().unwrap() -= 1;
                }
            }
            MainToWorkerMessage::Reset(barrier) => {
                #[cfg(debug_assertions)]
                println!("Worker {} resetting!", id);

                // return the indexed documents...
                sndr.send(WorkerToMainMessage {
                    id,
                    block_index_results: Some(doc_miner.get_results()),
                })
                .expect("Failed to send message back to main thread!");

                barrier.wait();
            }
            MainToWorkerMessage::Synchronize(barrier) => {
                barrier.wait();
            }
            MainToWorkerMessage::Decode { n, postings_stream_reader, postings_stream_decoders } => {
                postings_stream_reader.decode_next_n(n, postings_stream_decoders, with_positions, &field_infos);
            }
            MainToWorkerMessage::Terminate => {
                break;
            }
        }
    }
}
