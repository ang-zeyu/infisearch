pub mod miner;

use crate::DocInfos;
use std::sync::Mutex;
use crate::spimireader::DocFieldForMerge;
use crate::spimireader::TermDocForMerge;
use crate::FieldInfos;
use dashmap::DashMap;
use crate::spimireader::PostingsStreamDecoder;
use crate::spimireader::PostingsStreamReader;
use std::path::PathBuf;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::str;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;

use byteorder::{ByteOrder, LittleEndian};
use rustc_hash::FxHashMap;

use miner::WorkerMiner;
use crate::spimiwriter;
use crate::utils::varint;

pub struct Worker {
    pub id: usize,
    pub join_handle: thread::JoinHandle<()>,
    pub tx: Sender<MainToWorkerMessage>
}

impl Worker {
    pub fn send_work(&self, doc_id: u32, field_texts: Vec<(String, String)>, field_store_path: PathBuf) {
        self.tx.send(MainToWorkerMessage::Index {
            doc_id,
            field_texts,
            field_store_path,
        }).expect("Failed to send work message to worker!");
    }

    pub fn receive_work(&self) {
        self.tx.send(MainToWorkerMessage::Reset).expect("Failed to request worker doc miner move!");
    }

    pub fn decode_spimi(
        &self,
        n: u32,
        postings_stream_reader: PostingsStreamReader,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
    ) {
        self.tx.send(MainToWorkerMessage::Decode {
            n,
            postings_stream_reader,
            postings_stream_decoders,
        }).expect("Failed to request worker spimi block decode!");
    }

    pub fn combine_and_sort_block(&self, worker_miners: Vec<WorkerMiner>, output_folder_path: PathBuf, block_number: u32, num_docs: u32, doc_infos: &Arc<Mutex<DocInfos>>) {
        self.tx.send(MainToWorkerMessage::Combine {
            worker_miners,
            output_folder_path,
            block_number,
            num_docs,
            doc_infos: Arc::clone(doc_infos),
        }).expect("Failed to send work message to worker!");
    }

    fn terminate(&self) {
        self.tx.send(MainToWorkerMessage::Terminate).expect("Failed to request worker termination!");
    }

    pub fn terminate_all_workers(workers: Vec<Worker>) {
        for worker in &workers {
            worker.terminate();
        }

        for worker in workers {
            worker.join_handle.join().expect("Failed to join worker.");
        }
    }

    pub fn make_available(&self) {
        self.tx.send(MainToWorkerMessage::Wait).unwrap_or_else(|_| panic!("Failed to send make_available message for worker {}!", self.id));
    }

    pub fn make_all_workers_available(workers: &[Worker]) {
        for worker in workers {
            worker.make_available();
        }
    }

    pub fn wait_on_all_workers(workers: &[Worker], rx_main: &Receiver<WorkerToMainMessage>, num_threads: u32) {
        for _i in 0..num_threads {
            let worker_msg = rx_main.recv();
            match worker_msg {
                Ok(worker_msg_unwrapped) => {
                    if let Some(_doc_miner_unwrapped) = worker_msg_unwrapped.doc_miner {
                        panic!("Received data from worker {} unexpectedly!", worker_msg_unwrapped.id);
                    }
                },
                Err(e) => panic!("Failed to receive idle message from worker! {}", e)
            }
        }

        Worker::make_all_workers_available(workers);
    }
    
    pub fn get_available_worker<'b> (workers: &'b [Worker], rx_main: &'b Receiver<WorkerToMainMessage>) -> &'b Worker {
        let worker_msg = rx_main.recv();
        match worker_msg {
            Ok(msg) => {
                return workers.get(msg.id).unwrap_or_else(|| panic!("Failed to return worker reference for index {}", msg.id));
            },
            Err(e) => panic!("Failed to receive message from worker @get_available_worker! {}", e)
        }
    }
}

pub enum MainToWorkerMessage {
    Reset,
    Wait,
    Terminate,
    Combine {
        worker_miners: Vec<WorkerMiner>,
        output_folder_path: PathBuf,
        block_number: u32,
        num_docs: u32,
        doc_infos: Arc<Mutex<DocInfos>>,
    },
    Index {
        doc_id: u32,
        field_texts: Vec<(String, String)>,
        field_store_path: PathBuf
    },
    Decode {
        n: u32,
        postings_stream_reader: PostingsStreamReader,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
    }
}

pub struct WorkerToMainMessage {
    pub id: usize,
    pub doc_miner: Option<WorkerMiner>,
}

pub fn worker (
    id: usize,
    sndr: Sender<WorkerToMainMessage>, 
    rcvr: Receiver<MainToWorkerMessage>,
    /* Immutable shared data structures... */
    field_infos: Arc<FieldInfos>,
    num_scored_fields: usize,
    expected_num_docs_per_reset: usize,
) {
    // Initialize data structures...
    let mut doc_miner = WorkerMiner {
        field_infos: Arc::clone(&field_infos),
        terms: FxHashMap::default(),
        num_scored_fields,
        document_lengths: Vec::with_capacity(expected_num_docs_per_reset),
    };

    let send_available_msg = || {
        sndr.send(WorkerToMainMessage {
            id,
            doc_miner: Option::None,
        }).expect("Failed to send availability message back to main thread!");
    };

    loop {
        let msg = rcvr.recv().expect("Failed to receive message on worker side!");
        match msg {
            MainToWorkerMessage::Wait => {
                send_available_msg();
            },
            MainToWorkerMessage::Index { doc_id, field_texts, field_store_path } => {
                doc_miner.index_doc(doc_id, field_texts, field_store_path);
        
                send_available_msg();
            },
            MainToWorkerMessage::Combine {
                worker_miners,
                output_folder_path,
                block_number,
                num_docs,
                doc_infos,
            } => {
                spimiwriter::combine_worker_results_and_write_block(worker_miners, doc_infos, output_folder_path, block_number, num_docs);
                println!("Worker {} wrote spimi block {}!", id, block_number);

                send_available_msg();
            },
            MainToWorkerMessage::Reset => {
                println!("Worker {} resetting!", id);
            
                // return the indexed documents...
                sndr.send(WorkerToMainMessage {
                    id,
                    doc_miner: Option::from(doc_miner),
                }).expect("Failed to send message back to main thread!");
                
                // reset local variables...
                doc_miner = WorkerMiner {
                    field_infos: Arc::clone(&field_infos),
                    terms: FxHashMap::default(),
                    num_scored_fields,
                    document_lengths: Vec::with_capacity(expected_num_docs_per_reset),
                };
            },
            MainToWorkerMessage::Decode {
                n,
                mut postings_stream_reader,
                postings_stream_decoders,
            } => {
                let mut u32_buf: [u8; 4] = [0; 4];
                let mut u8_buf: [u8; 1] = [0; 1];
                
                for _unused in 0..n {
                    if let Ok(()) = postings_stream_reader.buffered_dict_reader.read_exact(&mut u32_buf) {
                        // Temporary combined dictionary table / dictionary string
                        let term_len: usize = LittleEndian::read_u32(&u32_buf) as usize;
                    
                        let mut term_vec = vec![0; term_len];
                        postings_stream_reader.buffered_dict_reader.read_exact(&mut term_vec).unwrap();
                        let term = str::from_utf8(&term_vec).unwrap().to_owned();
        
                        postings_stream_reader.buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                        let doc_freq = LittleEndian::read_u32(&u32_buf);
        
                        postings_stream_reader.buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                        let pl_offset = LittleEndian::read_u32(&u32_buf);
        
                        // Postings list
                        postings_stream_reader.buffered_reader.seek(SeekFrom::Start(pl_offset as u64)).unwrap();
        
                        let mut term_docs: Vec<TermDocForMerge> = Vec::with_capacity(doc_freq as usize);
                        for _i in 0..doc_freq {
                            postings_stream_reader.buffered_reader.read_exact(&mut u32_buf).unwrap();
                            let doc_id = LittleEndian::read_u32(&u32_buf);
        
                            postings_stream_reader.buffered_reader.read_exact(&mut u8_buf).unwrap();
                            let num_fields = u8_buf[0];
        
                            let mut doc_fields: Vec<DocFieldForMerge> = Vec::with_capacity(num_fields as usize);
                            for _j in 0..num_fields {
                                postings_stream_reader.buffered_reader.read_exact(&mut u8_buf).unwrap();
                                let field_id = u8_buf[0];
                                postings_stream_reader.buffered_reader.read_exact(&mut u32_buf).unwrap();
                                let field_tf = LittleEndian::read_u32(&u32_buf);

                                /*
                                 Pre-encode field tf and position gaps into varint in the worker,
                                 then write it out in the main thread later.
                                */
                                let mut field_tf_and_positions_varint: Vec<u8> = Vec::with_capacity(4 + field_tf as usize * 2);

                                varint::get_var_int_vec(field_tf, &mut field_tf_and_positions_varint);
                                
                                let mut prev_pos = 0;
                                for _k in 0..field_tf {
                                    postings_stream_reader.buffered_reader.read_exact(&mut u32_buf).unwrap();
                                    let curr_pos = LittleEndian::read_u32(&u32_buf);
                                    varint::get_var_int_vec(curr_pos - prev_pos, &mut field_tf_and_positions_varint);
                                    prev_pos = curr_pos;
                                }
        
                                doc_fields.push(DocFieldForMerge {
                                    field_id,
                                    field_tf,
                                    field_tf_and_positions_varint,
                                });
                            }
        
                            term_docs.push(TermDocForMerge {
                                doc_id,
                                doc_fields
                            });
                        }

                        postings_stream_reader.future_term_buffer.push_back((term, term_docs));
                    } else {
                        break; // eof
                    }
                }

                {
                    let mut postings_stream_decoder_entry = postings_stream_decoders.get_mut(&postings_stream_reader.idx).unwrap();
                    let postings_stream_decoder = postings_stream_decoder_entry.value_mut();
                    match postings_stream_decoder {
                        PostingsStreamDecoder::None => {
                            *postings_stream_decoder = PostingsStreamDecoder::Reader(postings_stream_reader);
                        },
                        PostingsStreamDecoder::Notifier(_tx) => {
                            let notifier_decoder = std::mem::replace(postings_stream_decoder, PostingsStreamDecoder::Reader(postings_stream_reader));
                            
                            // Main thread was blocked as this worker was still decoding
                            // Re-notify that decoding is done!
                            if let PostingsStreamDecoder::Notifier(tx) = notifier_decoder {
                                tx.lock().unwrap().send(()).unwrap();
                            }
                        },
                        PostingsStreamDecoder::Reader(_r) => panic!("Reader still available in array @worker")
                    }
                }

                send_available_msg();
            },
            MainToWorkerMessage::Terminate => {
                break;
            },
        }
    }
}
