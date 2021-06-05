pub mod miner;

use std::sync::Barrier;
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
use std::thread;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam::Sender;
use crossbeam::Receiver;
use rustc_hash::FxHashMap;

use miner::WorkerMiner;
use crate::spimiwriter;
use crate::utils::varint;

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

    pub fn wait_on_all_workers(tx_main: &Sender<MainToWorkerMessage>, num_threads: u32) {
        let receive_work_barrier: Arc<Barrier> = Arc::new(Barrier::new((num_threads + 1) as usize));
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
    expected_num_docs_per_reset: usize,
) {
    // Initialize data structures...
    let mut doc_miner = WorkerMiner {
        field_infos: Arc::clone(&field_infos),
        terms: FxHashMap::default(),
        document_lengths: Vec::with_capacity(expected_num_docs_per_reset),
    };

    loop {
        let msg = rcvr.recv().expect("Failed to receive message on worker side!");
        match msg {
            MainToWorkerMessage::Index { doc_id, field_texts, field_store_path } => {
                doc_miner.index_doc(doc_id, field_texts, field_store_path);
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
            },
            MainToWorkerMessage::Reset(barrier) => {
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
                    document_lengths: Vec::with_capacity(expected_num_docs_per_reset),
                };

                barrier.wait();
            },
            MainToWorkerMessage::Synchronize(barrier) => {
                barrier.wait();
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

                        let mut max_doc_term_score: f32 = 0.0;
        
                        let mut term_docs: Vec<TermDocForMerge> = Vec::with_capacity(doc_freq as usize);
                        for _i in 0..doc_freq {
                            postings_stream_reader.buffered_reader.read_exact(&mut u32_buf).unwrap();
                            let doc_id = LittleEndian::read_u32(&u32_buf);
        
                            postings_stream_reader.buffered_reader.read_exact(&mut u8_buf).unwrap();
                            let num_fields = u8_buf[0];
        
                            let mut curr_doc_term_score: f32 = 0.0;

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

                                let field_info = field_infos.field_infos_by_id.get(field_id as usize).unwrap();
                                let k = field_info.k;
                                let b = field_info.b;
                                curr_doc_term_score += (field_tf as f32 * (k + 1.0))
                                    / (field_tf as f32
                                        + k * (
                                            1.0
                                            - b
                                            + b * (postings_stream_reader.doc_infos_unlocked.get_field_len_factor(doc_id as usize, field_id as usize))
                                        )
                                    )
                                    * field_info.weight;
        
                                doc_fields.push(DocFieldForMerge {
                                    field_id,
                                    field_tf_and_positions_varint,
                                });
                            }

                            if curr_doc_term_score > max_doc_term_score {
                                max_doc_term_score = curr_doc_term_score;
                            }
        
                            term_docs.push(TermDocForMerge {
                                doc_id,
                                doc_fields
                            });
                        }

                        postings_stream_reader.future_term_buffer.push_back((term, term_docs, max_doc_term_score));
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
            },
            MainToWorkerMessage::Terminate => {
                break;
            },
        }
    }
}
