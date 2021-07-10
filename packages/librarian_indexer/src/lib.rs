
mod docinfo;
mod fieldinfo;
mod spimireader;
mod spimiwriter;
mod utils;
mod worker;

use crate::docinfo::DocInfos;
use crate::worker::MainToWorkerMessage;
use crate::worker::WorkerToMainMessage;
use crate::worker::Worker;
use crate::fieldinfo::FieldInfo;
use crate::fieldinfo::FieldInfos;
use rustc_hash::FxHashMap;

use std::cmp::Ordering;
use std::sync::Mutex;
use std::fs;
use std::time::Instant;
use std::sync::Arc;
use std::path::Path;
use std::path::PathBuf;

use crossbeam::Sender;
use crossbeam::Receiver;

#[macro_use]
extern crate lazy_static;


pub struct Indexer {
    num_docs: u32,
    num_threads: u32,
    expected_num_docs_per_thread: usize,
    doc_id_counter: u32,
    spimi_counter: u32,
    field_store_block_size: u32,
    field_infos_temp: FxHashMap<String, FieldInfo>,
    field_infos: Option<Arc<FieldInfos>>,
    output_folder_path: PathBuf,
    workers: Vec<Worker>,
    doc_infos: Option<Arc<Mutex<DocInfos>>>,
    tx_main: Sender<MainToWorkerMessage>,
    rx_main: Receiver<WorkerToMainMessage>,
    tx_worker: Sender<WorkerToMainMessage>,
    rx_worker: Receiver<MainToWorkerMessage>,
    num_workers_writing_blocks: Arc<Mutex<usize>>,
}

pub struct FieldConfig {
    pub name: &'static str,
    pub do_store: bool,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
}

impl Indexer {
    pub fn new(output_folder_path: &Path, num_docs: u32, num_threads: u32) -> Indexer {
        let (tx_worker, rx_main) : (Sender<WorkerToMainMessage>, Receiver<WorkerToMainMessage>) = crossbeam::bounded(num_threads as usize);
        let (tx_main, rx_worker) : (Sender<MainToWorkerMessage>, Receiver<MainToWorkerMessage>) = crossbeam::bounded(num_threads as usize);

        Indexer {
            num_docs,
            num_threads,
            expected_num_docs_per_thread: (num_docs / num_threads * 2) as usize,
            doc_id_counter: 0,
            spimi_counter: 0,
            field_store_block_size: 1,
            field_infos_temp: FxHashMap::default(),
            field_infos: Option::None,
            output_folder_path: output_folder_path.to_path_buf(),
            workers: Vec::with_capacity(num_threads as usize),
            doc_infos: Option::None,
            tx_main,
            rx_main,
            tx_worker,
            rx_worker,
            num_workers_writing_blocks: Arc::from(Mutex::from(0)),
        }
    }

    pub fn add_field(&mut self, field_config: FieldConfig) {
        self.field_infos_temp.insert(
            field_config.name.to_owned(),
            FieldInfo {
                id: 0,
                do_store: field_config.do_store,
                weight: field_config.weight,
                k: field_config.k,
                b: field_config.b,
            }
        );
    }

    pub fn set_field_store_block_size(&mut self, size: u32) {
        self.field_store_block_size = size;
    }

    pub fn finalise_fields(&mut self) {
        let mut field_entries: Vec<(&String, &mut FieldInfo)> = self.field_infos_temp.iter_mut().collect();
        field_entries.sort_by(|a, b| {
            if a.1.weight < b.1.weight {
                Ordering::Greater
            } else if a.1.weight > b.1.weight {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });

        for (field_id, tup) in field_entries.iter_mut().enumerate() {
            tup.1.id = field_id as u8;
        }

        let field_infos = FieldInfos::init(std::mem::take(&mut self.field_infos_temp), self.field_store_block_size, &self.output_folder_path);
        field_infos.dump(&self.output_folder_path);
        
        self.doc_infos = Option::from(
            Arc::from(Mutex::from(DocInfos::init_doc_infos(field_infos.num_scored_fields)))
        );
        
        let field_infos_arc: Arc<FieldInfos> = Arc::new(field_infos);
        
        let num_docs_per_thread = self.expected_num_docs_per_thread;
        for i in 0..self.num_threads {
            let tx_worker_clone = self.tx_worker.clone();
            let rx_worker_clone = self.rx_worker.clone();
            let field_info_clone = Arc::clone(&field_infos_arc);
            let num_workers_writing_blocks_clone = Arc::clone(&self.num_workers_writing_blocks);

            self.workers.push(Worker {
                id: i as usize,
                join_handle: std::thread::spawn(move ||
                    worker::worker(i as usize, tx_worker_clone, rx_worker_clone, field_info_clone, num_docs_per_thread, num_workers_writing_blocks_clone)),
            });
        }

        self.field_infos = Option::from(field_infos_arc);
    }

    fn block_number(&self) -> u32 {
        ((self.doc_id_counter as f64) / (self.num_docs as f64)).ceil() as u32
    }

    pub fn index_document(&mut self, field_texts: Vec<(&'static str, String)>) {
        self.tx_main.send(MainToWorkerMessage::Index {
            doc_id: self.doc_id_counter,
            field_texts,
        }).expect("Failed to send work message to worker!");
    
        self.doc_id_counter += 1;
        self.spimi_counter += 1;

        if self.spimi_counter == self.num_docs {
            self.write_block();
        }
    }

    pub fn index_html_document(&mut self, link: String, html_text: String) {
        self.tx_main.send(MainToWorkerMessage::IndexHtml {
            doc_id: self.doc_id_counter,
            link,
            html_text,
        }).expect("Failed to send html work message to worker!");
    
        self.doc_id_counter += 1;
        self.spimi_counter += 1;

        if self.spimi_counter == self.num_docs {
            self.write_block();
        }
    }

    pub fn finish_writing_docs(mut self, instant: Option<Instant>) {
        if self.spimi_counter != 0 && self.spimi_counter != self.num_docs {
            println!("Writing last spimi block");
            self.write_block();
        }
        
        // Wait on all workers
        Worker::wait_on_all_workers(&self.tx_main, self.num_threads);
        println!("Number of docs: {}", self.doc_id_counter);
        if let Some(now) = instant {
            print_time_elapsed(now, "Block indexing done!");
        }

        // Merge spimi blocks
        // Go through all blocks at once
        spimireader::merge_blocks(self.doc_id_counter, self.block_number(), self.doc_infos.unwrap(), &self.tx_main, &self.output_folder_path);

        if let Some(now) = instant {
            print_time_elapsed(now, "Blocks merged!");
        }
        Worker::terminate_all_workers(self.workers, self.tx_main); 
    }
}

fn print_time_elapsed(instant: Instant, extra_message: &str) {
    let elapsed = instant.elapsed().as_secs_f64();
    println!("{} {} mins {} seconds elapsed.", extra_message, (elapsed as u32) / 60, elapsed % 60.0);
}
