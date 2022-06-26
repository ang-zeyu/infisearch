use std::sync::{Arc, Barrier};

use crossbeam::channel::Sender;

use crate::worker::{MainToWorkerMessage, Worker};
use super::Indexer;

impl Indexer {
    pub fn terminate_all_workers(tx_main: Sender<MainToWorkerMessage>, workers: Vec<Worker>) {
        drop(tx_main);

        for worker in workers {
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
