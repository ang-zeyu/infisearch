pub mod miner;

use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;

use byteorder::{ByteOrder, LittleEndian};

use crate::FieldInfo;
use miner::WorkerMiner;
use crate::spimiwriter;
use crate::worker::miner::DocField;
use crate::worker::miner::TermDoc;

pub struct Worker<'a> {
    pub id: usize,
    pub join_handle: thread::JoinHandle<()>,
    pub tx: Sender<MainToWorkerMessage<'a>>
}

impl<'a> Worker<'a> {
    pub fn send_work(&self, doc_id: u32, field_texts: Vec<(String, String)>) {
        self.tx.send(MainToWorkerMessage::Index {
            doc_id,
            field_texts
        }).expect("Failed to send work message to worker!");
    }

    pub fn receive_work(&self) {
        self.tx.send(MainToWorkerMessage::Reset).expect("Failed to request worker doc miner move!");
    }

    pub fn combine_and_sort_block(&self, worker_miners: Vec<WorkerMiner>, output_folder_path: PathBuf, block_number: u32) {
        self.tx.send(MainToWorkerMessage::Combine {
            worker_miners,
            output_folder_path,
            block_number
        }).expect("Failed to send work message to worker!");
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

    pub fn make_all_workers_available(workers: &'a mut Vec<Worker>) {
        for worker in workers {
            worker.tx.send(MainToWorkerMessage::Wait).expect("Failed to send make_all_workers_available message!");
        }
    }
    
    pub fn get_available_worker<'b> (workers: &'b mut Vec<Worker<'a>>, rx_main: &Receiver<WorkerToMainMessage>) -> &'b mut Worker<'a> {
        let worker_msg = rx_main.recv();
        match worker_msg {
            Ok(msg) => {
                return workers.get_mut(msg.id).expect(&format!("Failed to return mutable worker reference for index {}", msg.id));
            },
            Err(e) => panic!("Failed to receive message from worker @get_available_worker! {}", e)
        }
    }
}

pub enum MainToWorkerMessage<'a> {
    Reset,
    Wait,
    Terminate,
    Combine {
        worker_miners: Vec<WorkerMiner>,
        output_folder_path: PathBuf,
        block_number: u32,
    },
    Index {
        doc_id: u32,
        field_texts: Vec<(String, String)>
    },
    Decode {
        n: u32,
        buffered_dict_reader: &'a mut BufReader<File>,
        buffered_reader: &'a mut BufReader<File>,
        curr_term_docs: &'a mut Vec<TermDoc>,
        future_term_buffer: &'a mut Vec<(String, Vec<TermDoc>)>,
    }
}

pub struct WorkerToMainMessage {
    pub id: usize,
    pub doc_miner: Option<WorkerMiner>
}

pub fn worker (
    id: usize,
    sndr: Sender<WorkerToMainMessage>, 
    rcvr: Receiver<MainToWorkerMessage>,
    /* Immutable shared data structures... */
    field_infos: Arc<HashMap<String, FieldInfo>>,
) {
    // Initialize data structures...
    let mut doc_miner = WorkerMiner {
        field_infos: Arc::clone(&field_infos),
        terms: HashMap::new()
    };

    let send_available_msg = || {
        sndr.send(WorkerToMainMessage {
            id,
            doc_miner: Option::None
        }).expect("Failed to send availability message back to main thread!");
    };

    loop {
        let msg = rcvr.recv().expect("Failed to receive message on worker side!");
        match msg {
            MainToWorkerMessage::Wait => {
                send_available_msg();
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
                doc_miner.index_doc(doc_id, field_texts);
        
                send_available_msg();
            },
            MainToWorkerMessage::Combine {
                worker_miners,
                output_folder_path,
                block_number,
            } => {
                spimiwriter::combine_worker_results_and_write_block(worker_miners, output_folder_path, block_number);
                println!("Worker {} wrote spimi block {}!", id, block_number);

                send_available_msg();
            },
            MainToWorkerMessage::Decode {
                n,
                buffered_dict_reader,
                buffered_reader,
                curr_term_docs,
                future_term_buffer
            } => {
                let mut u32_buf: [u8; 4] = [0; 4];
                let mut u8_buf: [u8; 1] = [0; 1];

                for a in 0..n {
                    buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                    let term_len = LittleEndian::read_u32(&u32_buf);
                    
                    let mut term_vec: Vec<u8> = vec![0; term_len as usize];
                    buffered_dict_reader.read_exact(&mut term_vec).unwrap();
                    let term = str::from_utf8(&term_vec).unwrap().to_owned();
    
                    buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                    let doc_freq = LittleEndian::read_u32(&u32_buf);
    
                    buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                    let pl_offset = LittleEndian::read_u32(&u32_buf);
    
                    buffered_reader.seek(SeekFrom::Start(pl_offset as u64)).unwrap();
    
                    let mut term_docs: Vec<TermDoc> = Vec::new();
                    for _i in 0..doc_freq {
                        buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                        let doc_id = LittleEndian::read_u32(&u32_buf);
    
                        buffered_dict_reader.read_exact(&mut u8_buf).unwrap();
                        let num_fields = u8_buf[0];
    
                        let mut doc_fields: Vec<DocField> = Vec::new();
                        for _j in 0..num_fields {
                            buffered_dict_reader.read_exact(&mut u8_buf).unwrap();
                            let field_id = u8_buf[0];
                            buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                            let field_tf = LittleEndian::read_u32(&u32_buf);
                            
                            let mut field_positions: Vec<u32> = Vec::new();
                            for _k in 0..field_tf {
                                buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                                field_positions.push(LittleEndian::read_u32(&u32_buf));
                            }
    
                            doc_fields.push(DocField {
                                field_id,
                                field_tf,
                                field_positions
                            });
                        }
    
                        term_docs.push(TermDoc {
                            doc_id,
                            doc_fields
                        });
                    }

                    future_term_buffer.push((term, term_docs));
                }

                send_available_msg();
            },
            MainToWorkerMessage::Terminate => {
                break;
            },
        }
    }
}
