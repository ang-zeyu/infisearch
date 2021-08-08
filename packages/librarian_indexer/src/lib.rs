
mod docinfo;
pub mod fieldinfo;
pub mod loader;
mod spimireader;
mod spimiwriter;
mod utils;
mod worker;

use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;
use std::sync::Arc;
use std::path::Path;
use std::path::PathBuf;

use librarian_common::LibrarianLanguageConfig;
use librarian_common::tokenize::Tokenizer;
use librarian_lang_chinese::chinese;
use librarian_lang_latin::english;

use crate::docinfo::DocInfos;
use crate::fieldinfo::FieldConfig;
use crate::fieldinfo::FieldsConfig;
use crate::fieldinfo::FieldInfo;
use crate::fieldinfo::FieldInfos;
use crate::loader::csv::CsvLoader;
use crate::loader::html::HtmlLoader;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::worker::MainToWorkerMessage;
use crate::worker::WorkerToMainMessage;
use crate::worker::Worker;

use crossbeam::Sender;
use crossbeam::Receiver;
use rustc_hash::FxHashMap;
use serde::{Serialize,Deserialize};

#[macro_use]
extern crate lazy_static;

fn get_default_num_threads() -> usize {
    std::cmp::max(num_cpus::get_physical() - 1, 1)
}

fn get_default_num_docs_per_block() -> u32 {
    1000
}

fn get_default_pl_cache_threshold() -> u32 {
    1048576
}

fn get_default_loader_configs() -> FxHashMap<String, serde_json::Value> {
    let mut configs = FxHashMap::default();

    configs.insert("HtmlLoader".to_owned(), serde_json::json!({}));

    configs
}

pub fn get_loaders_from_config(config: &mut LibrarianConfig) -> Vec<Box<dyn Loader>> {
    let mut loaders: Vec<Box<dyn Loader>> = Vec::new();

    for (key, value) in std::mem::take(&mut config.indexing_config.loader_configs) {
        match &key[..] {
            "HtmlLoader" => {
                loaders.push(Box::new(HtmlLoader {
                    options: serde_json::from_value(value).expect("HtmlLoader options did not match schema!"),
                }))
            },
            "CsvLoader" => {
                loaders.push(Box::new(CsvLoader {
                    options: serde_json::from_value(value).expect("CsvLoader options did not match schema!"),
                }))
            },
            _ => panic!("Unknown loader type encountered in config")
        }
    }

    loaders
}

fn get_default_num_pls_per_dir() -> u32 {
    1000
}

fn get_default_with_positions() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct LibrarianIndexingConfig {
    #[serde(default = "get_default_num_threads")]
    #[serde(skip_serializing)]
    num_threads: usize,

    #[serde(default = "get_default_num_docs_per_block")]
    #[serde(skip_serializing)]
    num_docs_per_block: u32,

    #[serde(default = "get_default_pl_cache_threshold")]
    #[serde(skip_serializing)]
    pl_cache_threshold: u32,

    #[serde(default = "get_default_loader_configs")]
    #[serde(skip_serializing)]
    loader_configs: FxHashMap<String, serde_json::Value>,

    #[serde(default = "Vec::new")]
    pl_names_to_cache: Vec<u32>,

    #[serde(default = "get_default_num_pls_per_dir")]
    num_pls_per_dir: u32,

    #[serde(default = "get_default_with_positions")]
    with_positions: bool,
}

impl Default for LibrarianIndexingConfig {
    fn default() -> Self {
        LibrarianIndexingConfig {
            num_threads: get_default_num_threads(),
            num_docs_per_block: get_default_num_docs_per_block(),
            pl_cache_threshold: get_default_pl_cache_threshold(),
            loader_configs: get_default_loader_configs(),
            pl_names_to_cache: Vec::new(),
            num_pls_per_dir: get_default_num_pls_per_dir(),
            with_positions: get_default_with_positions(),
        }
    }
}

#[derive(Deserialize)]
pub struct LibrarianConfig {
    #[serde(default)]
    indexing_config: LibrarianIndexingConfig,
    #[serde(default)]
    language: LibrarianLanguageConfig,
    fields_config: FieldsConfig,
}

#[derive(Serialize)]
pub struct LibrarianOutputConfig<'a> {
    indexing_config: &'a LibrarianIndexingConfig,
    language: &'a LibrarianLanguageConfig,
    field_infos: &'a FieldInfos,
}

impl Default for LibrarianConfig {
    fn default() -> Self {

        LibrarianConfig {
            indexing_config: LibrarianIndexingConfig::default(),
            language: LibrarianLanguageConfig {
                lang: "english".to_owned(),
                options: Option::None,
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
    indexing_config: LibrarianIndexingConfig,
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
    language_config: LibrarianLanguageConfig,
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
        let num_threads = config.indexing_config.num_threads;

        let tokenizer = Indexer::resolve_tokenizer(&config.language);

        let mut indexer = Indexer {
            indexing_config: config.indexing_config,
            expected_num_docs_per_thread,
            doc_id_counter: 0,
            spimi_counter: 0,
            field_store_block_size: config.fields_config.field_store_block_size,
            field_infos: Option::None,
            output_folder_path: output_folder_path.to_path_buf(),
            workers: Vec::with_capacity(num_threads),
            doc_infos: Option::None,
            tx_main,
            rx_main,
            tx_worker,
            rx_worker,
            num_workers_writing_blocks: Arc::from(Mutex::from(0)),
            tokenizer,
            language_config: config.language,
        };

        let mut field_infos_by_name: FxHashMap<String, FieldInfo> = FxHashMap::default();
        for field_config in config.fields_config.fields {
            indexer.add_field(&mut field_infos_by_name, field_config);
        }
        indexer.finalise_fields(field_infos_by_name);

        indexer
    }

    fn resolve_tokenizer(language_config: &LibrarianLanguageConfig) -> Arc<dyn Tokenizer + Send + Sync> {
        match language_config.lang.as_str() {
            "latin" => {
                if let Some(options) = language_config.options.as_ref() {
                    Arc::new(english::new_with_options(serde_json::from_value(options.clone()).unwrap()))
                } else {
                    Arc::new(english::EnglishTokenizer::default())
                }
            },
            "chinese" => {
                if let Some(options) = language_config.options.as_ref() {
                    Arc::new(chinese::new_with_options(serde_json::from_value(options.clone()).unwrap()))
                } else {
                    Arc::new(chinese::ChineseTokenizer::default())
                }
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
        self.field_infos = Option::from(Arc::new(field_infos));
        
        self.doc_infos = Option::from(
            Arc::from(Mutex::from(DocInfos::init_doc_infos(self.field_infos.as_ref().unwrap().num_scored_fields)))
        );
        
        // Construct worker threads
        let num_docs_per_thread = self.expected_num_docs_per_thread;
        let with_positions = self.indexing_config.with_positions;
        for i in 0..self.indexing_config.num_threads {
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
                        with_positions,
                        num_docs_per_thread,
                        num_workers_writing_blocks_clone,
                    )),
            });
        }
    }

    fn block_number(&self) -> u32 {
        ((self.doc_id_counter as f64) / (self.indexing_config.num_docs_per_block as f64)).ceil() as u32
    }

    pub fn index_document(&mut self, loader_result: Box<dyn LoaderResult + Send>) {
        self.tx_main.send(MainToWorkerMessage::Index {
            doc_id: self.doc_id_counter,
            loader_result,
        }).expect("Failed to send work message to worker!");
    
        self.doc_id_counter += 1;
        self.spimi_counter += 1;

        if self.spimi_counter == self.indexing_config.num_docs_per_block {
            self.write_block();
        }
    }

    fn write_librarian_config(&self) {
        let serialized = serde_json::to_string(&LibrarianOutputConfig {
            indexing_config: &self.indexing_config,
            language: &self.language_config,
            field_infos: self.field_infos.as_ref().unwrap(),
        }).unwrap();

        File::create(self.output_folder_path.join("_librarian_config.json"))
            .unwrap()
            .write_all(serialized.as_bytes())
            .unwrap();
    }

    pub fn finish_writing_docs(mut self, instant: Option<Instant>) {
        if self.spimi_counter != 0 && self.spimi_counter != self.indexing_config.num_docs_per_block {
            println!("Writing last spimi block");
            self.write_block();
        }
        
        // Wait on all workers
        Worker::wait_on_all_workers(&self.tx_main, self.indexing_config.num_threads);
        println!("Number of docs: {}", self.doc_id_counter);
        if let Some(now) = instant {
            print_time_elapsed(now, "Block indexing done!");
        }

        // Merge spimi blocks
        // Go through all blocks at once
        let num_blocks = self.block_number();
        spimireader::merge_blocks(
            self.doc_id_counter,
            num_blocks,
            &mut self.indexing_config,
            std::mem::take(&mut self.doc_infos).unwrap(),
            &self.tx_main,
            &self.output_folder_path
        );

        self.write_librarian_config();

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
