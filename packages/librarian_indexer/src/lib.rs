
mod docinfo;
pub mod fieldinfo;
mod spimireader;
mod spimiwriter;
mod utils;
mod worker;

use librarian_common::tokenize::Tokenizer;
use librarian_common::tokenize::english::EnglishTokenizer;

use crate::docinfo::DocInfos;
use crate::fieldinfo::FieldConfig;
use crate::fieldinfo::FieldsConfig;
use crate::worker::MainToWorkerMessage;
use crate::worker::WorkerToMainMessage;
use crate::worker::Worker;
use crate::fieldinfo::FieldInfo;
use crate::fieldinfo::FieldInfos;
use rustc_hash::FxHashMap;

use std::cmp::Ordering;
use std::sync::Mutex;
use std::time::Instant;
use std::sync::Arc;
use std::path::Path;
use std::path::PathBuf;

use crossbeam::Sender;
use crossbeam::Receiver;
use serde::Deserialize;

#[macro_use]
extern crate lazy_static;

fn get_default_num_threads() -> usize {
    std::cmp::max(num_cpus::get_physical() - 1, 1)
}

fn get_default_num_docs_per_block() -> u32 {
    1000
}

#[derive(Deserialize)]
struct LibrarianIndexingConfig {
    #[serde(default = "get_default_num_threads")]
    num_threads: usize,
    #[serde(default = "get_default_num_docs_per_block")]
    num_docs_per_block: u32,
}

impl Default for LibrarianIndexingConfig {
    fn default() -> Self {
        LibrarianIndexingConfig {
            num_threads: get_default_num_threads(),
            num_docs_per_block: get_default_num_docs_per_block(),
        }
    }
}

#[derive(Deserialize)]
struct LibrarianLanguageConfig {
    lang: String,
}

#[derive(Deserialize)]
pub struct LibrarianConfig {
    #[serde(default)]
    indexing_config: LibrarianIndexingConfig,
    language: LibrarianLanguageConfig,
    fields_config: FieldsConfig,
}

impl Default for LibrarianConfig {
    fn default() -> Self {

        LibrarianConfig {
            indexing_config: LibrarianIndexingConfig::default(),
            language: LibrarianLanguageConfig {
                lang: "english".to_owned()
            },
            fields_config: FieldsConfig {
                field_store_block_size: 1,
                fields: vec![
                    FieldConfig {
                        name: "title".to_owned(),
                        do_store: false,
                        weight: 0.2,
                        k: 1.2,
                        b: 0.25
                    },
                    FieldConfig {
                        name: "heading".to_owned(),
                        do_store: false,
                        weight: 0.3,
                        k: 1.2,
                        b: 0.3
                    },
                    FieldConfig {
                        name: "body".to_owned(),
                        do_store: false,
                        weight: 0.5,
                        k: 1.2,
                        b: 0.75
                    },
                    FieldConfig {
                        name: "headingLink".to_owned(),
                        do_store: false,
                        weight: 0.0,
                        k: 1.2,
                        b: 0.75
                    },
                    FieldConfig {
                        name: "link".to_owned(),
                        do_store: true,
                        weight: 0.0,
                        k: 1.2,
                        b: 0.75
                    },
                ]
            }
        }
    }
}


pub struct Indexer {
    num_docs: u32,
    num_threads: usize,
    expected_num_docs_per_thread: usize,
    doc_id_counter: u32,
    spimi_counter: u32,
    field_store_block_size: u32,
    field_infos: Option<Arc<FieldInfos>>,
    output_folder_path: PathBuf,
    workers: Vec<Worker>,
    doc_infos: Option<Arc<Mutex<DocInfos>>>,
    tx_main: Sender<MainToWorkerMessage>,
    rx_main: Receiver<WorkerToMainMessage>,
    tx_worker: Sender<WorkerToMainMessage>,
    rx_worker: Receiver<MainToWorkerMessage>,
    num_workers_writing_blocks: Arc<Mutex<usize>>,
    tokenizer: Arc<dyn Tokenizer + Send + Sync>,
}


impl Indexer {
    pub fn new(
        output_folder_path: &Path,
        config: LibrarianConfig,
    ) -> Indexer {
        let (tx_worker, rx_main) : (Sender<WorkerToMainMessage>, Receiver<WorkerToMainMessage>) = crossbeam::bounded(config.indexing_config.num_threads);
        let (tx_main, rx_worker) : (Sender<MainToWorkerMessage>, Receiver<MainToWorkerMessage>) = crossbeam::bounded(config.indexing_config.num_threads);

        let expected_num_docs_per_thread = (
            config.indexing_config.num_docs_per_block / (config.indexing_config.num_threads as u32) * 2
        ) as usize;

        let tokenizer = Indexer::resolve_tokenizer(config.language);

        let mut indexer = Indexer {
            num_docs: config.indexing_config.num_docs_per_block,
            num_threads: config.indexing_config.num_threads,
            expected_num_docs_per_thread,
            doc_id_counter: 0,
            spimi_counter: 0,
            field_store_block_size: config.fields_config.field_store_block_size,
            field_infos: Option::None,
            output_folder_path: output_folder_path.to_path_buf(),
            workers: Vec::with_capacity(config.indexing_config.num_threads as usize),
            doc_infos: Option::None,
            tx_main,
            rx_main,
            tx_worker,
            rx_worker,
            num_workers_writing_blocks: Arc::from(Mutex::from(0)),
            tokenizer,
        };

        let mut field_infos_by_name: FxHashMap<String, FieldInfo> = FxHashMap::default();
        for field_config in config.fields_config.fields {
            indexer.add_field(&mut field_infos_by_name, field_config);
        }
        indexer.finalise_fields(field_infos_by_name);

        indexer
    }

    fn resolve_tokenizer(language_config: LibrarianLanguageConfig) -> Arc<dyn Tokenizer + Send + Sync> {
        match language_config.lang.as_str() {
            "english" => {
                Arc::new(EnglishTokenizer::default())
            },
            _ => {
                panic!("Unsupported language {}", language_config.lang)
            }
        }
    }

    fn add_field(&mut self, field_infos_by_name: &mut FxHashMap<String, FieldInfo>, field_config: FieldConfig) {
        field_infos_by_name.insert(
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

    fn finalise_fields(&mut self, mut field_infos_by_name: FxHashMap<String, FieldInfo>) {
        let mut field_entries: Vec<(&String, &mut FieldInfo)> = field_infos_by_name.iter_mut().collect();
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

        let field_infos = FieldInfos::init(field_infos_by_name, self.field_store_block_size, &self.output_folder_path);
        field_infos.dump(&self.output_folder_path);
        self.field_infos = Option::from(Arc::new(field_infos));
        
        self.doc_infos = Option::from(
            Arc::from(Mutex::from(DocInfos::init_doc_infos(self.field_infos.as_ref().unwrap().num_scored_fields)))
        );
        
        // Construct worker threads
        let num_docs_per_thread = self.expected_num_docs_per_thread;
        for i in 0..self.num_threads {
            let tx_worker_clone = self.tx_worker.clone();
            let rx_worker_clone = self.rx_worker.clone();
            let tokenize_clone = Arc::clone(&self.tokenizer);
            let field_info_clone = Arc::clone(self.field_infos.as_ref().unwrap());
            let num_workers_writing_blocks_clone = Arc::clone(&self.num_workers_writing_blocks);

            self.workers.push(Worker {
                id: i as usize,
                join_handle: std::thread::spawn(move ||
                    worker::worker(
                        i as usize,
                        tx_worker_clone,
                        rx_worker_clone,
                        tokenize_clone,
                        field_info_clone,
                        num_docs_per_thread,
                        num_workers_writing_blocks_clone,
                    )),
            });
        }
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
        let num_blocks = self.block_number();
        spimireader::merge_blocks(self.doc_id_counter, num_blocks, self.doc_infos.unwrap(), &self.tx_main, &self.output_folder_path);

        spimireader::cleanup_blocks(num_blocks, &self.output_folder_path);

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
