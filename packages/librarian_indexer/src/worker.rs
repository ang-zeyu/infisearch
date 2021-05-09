pub mod miner;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;

use crate::Dictionary;
use crate::FieldInfo;
use miner::WorkerMiner;

pub struct Worker {
    pub id: usize,
    pub join_handle: thread::JoinHandle<()>,
    pub tx: Sender<MainToWorkerMessage>
}

impl Worker {
    pub fn send_work(&self, doc_id: u32, field_texts: Vec<(String, String)>) {
        self.tx.send(MainToWorkerMessage::Index {
            doc_id,
            field_texts
        }).expect("Failed to send work message to worker!");
    }

    pub fn receive_work(&self) {
        self.tx.send(MainToWorkerMessage::Reset).expect("Failed to request worker doc miner move!");
    }

    fn terminate(&self) {
        self.tx.send(MainToWorkerMessage::Terminate).expect("Failed to request worker termination!");
    }

    pub fn terminate_all_workers(workers: Vec<Worker>) {
        for worker in &workers {
            &worker.terminate();
        }

        for worker in workers {
            worker.join_handle.join().expect("Failed to join worker.");
        }
    }

    pub fn make_all_workers_available<'a> (workers: &'a mut Vec<Worker>) {
        for worker in workers {
            worker.tx.send(MainToWorkerMessage::Wait).expect("Failed to send make_all_workers_available message!");
        }
    }
    
    pub fn get_available_worker<'a> (workers: &'a mut Vec<Worker>, rx_main: &Receiver<WorkerToMainMessage>) -> &'a mut Worker {
        let worker_msg = rx_main.recv();
        match worker_msg {
            Ok(msg) => {
                return workers.get_mut(msg.id).expect(&format!("Failed to return mutable worker reference for index {}", msg.id));
            },
            Err(e) => panic!("Failed to receive message from worker @get_available_worker! {}", e)
        }
    }
}

pub enum MainToWorkerMessage {
    Reset,
    Wait,
    Terminate,
    Index {
        doc_id: u32,
        field_texts: Vec<(String, String)>
    }
}

pub struct WorkerToMainMessage {
    pub id: usize,
    pub doc_miner: Option<WorkerMiner>
}

pub fn worker<'a> (
    id: usize,
    sndr: Sender<WorkerToMainMessage>, 
    rcvr: Receiver<MainToWorkerMessage>,
    /* Immutable shared data structures... */
    field_infos: Arc<HashMap<String, FieldInfo>>,
    /* Shared data structures... */
    dictionary: Arc<Dictionary<'a>>
) {
    // Initialize data structures...
    let mut doc_miner = WorkerMiner {
        field_infos: Arc::clone(&field_infos),
        terms: HashMap::new()
    };

    loop {
        let msg = rcvr.recv().expect("Failed to receive message on worker side!");
        match msg {
            MainToWorkerMessage::Wait => {
                sndr.send(WorkerToMainMessage {
                    id,
                    doc_miner: Option::None
                }).expect("Failed to send message back to main thread!");
    
                continue;
            },
            MainToWorkerMessage::Reset => {
                println!("Worker {} resetting!", id);
            
                // return the indexed documents...
                sndr.send(WorkerToMainMessage {
                    id,
                    doc_miner: Option::from(doc_miner)
                }).expect("Failed to send message back to main thread!");
                
                // reset local variables...
                doc_miner = WorkerMiner {
                    field_infos: Arc::clone(&field_infos),
                    terms: HashMap::new()
                };
    
                continue;
            },
            MainToWorkerMessage::Index { doc_id, field_texts } => {
                doc_miner.index_doc(doc_id, field_texts, &dictionary);
        
                sndr.send(WorkerToMainMessage {
                    id,
                    doc_miner: Option::None
                }).expect("Failed to send message back to main thread!");
            },
            MainToWorkerMessage::Terminate => {
                break;
            },
        }
    }
}
