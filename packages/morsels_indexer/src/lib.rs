
mod docinfo;
mod dynamic_index_info;
pub mod fieldinfo;
pub mod loader;
mod spimireader;
mod spimiwriter;
mod utils;
mod worker;

use std::time::{SystemTime, UNIX_EPOCH};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Instant;
use std::sync::Arc;
use std::path::Path;
use std::path::PathBuf;

use morsels_common::dictionary::Dictionary;
use morsels_common::MorselsLanguageConfig;
use morsels_common::tokenize::Tokenizer;
use morsels_lang_chinese::chinese;
use morsels_lang_latin::english;

use crate::docinfo::DocInfos;
use crate::dynamic_index_info::{DynamicIndexInfo, DYNAMIC_INDEX_INFO_FILE_NAME};
use crate::fieldinfo::FieldsConfig;
use crate::fieldinfo::FieldInfo;
use crate::fieldinfo::FieldInfos;
use crate::loader::csv::CsvLoader;
use crate::loader::html::HtmlLoader;
use crate::loader::json::JsonLoader;
use crate::loader::Loader;
use crate::worker::MainToWorkerMessage;
use crate::worker::WorkerToMainMessage;
use crate::worker::Worker;

use crossbeam::Sender;
use crossbeam::Receiver;
use glob::Pattern;
use rustc_hash::FxHashMap;
use serde::{Serialize,Deserialize};

#[macro_use]
extern crate lazy_static;


lazy_static! {
    static ref CURRENT_MILLIS: u128 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
}

fn get_default_num_threads() -> usize {
    std::cmp::max(num_cpus::get_physical() - 1, 1)
}

fn get_default_num_docs_per_block() -> u32 {
    1000
}

fn get_default_pl_cache_threshold() -> u32 {
    1048576
}

fn get_default_exclude_patterns() -> Vec<String> {
    vec!["_morsels_config.json".to_owned()]
}

fn get_default_loader_configs() -> FxHashMap<String, serde_json::Value> {
    let mut configs = FxHashMap::default();

    configs.insert("HtmlLoader".to_owned(), serde_json::json!({}));

    configs
}

fn get_default_num_pls_per_dir() -> u32 {
    1000
}

fn get_default_num_field_stores_per_dir() -> u32 {
    1000
}

fn get_default_with_positions() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct MorselsIndexingConfig {
    #[serde(default = "get_default_num_threads")]
    num_threads: usize,

    #[serde(default = "get_default_num_docs_per_block")]
    num_docs_per_block: u32,

    #[serde(default = "get_default_pl_cache_threshold")]
    pl_cache_threshold: u32,

    #[serde(default = "get_default_exclude_patterns")]
    exclude: Vec<String>,

    #[serde(default = "get_default_loader_configs")]
    loader_configs: FxHashMap<String, serde_json::Value>,

    #[serde(default = "get_default_num_pls_per_dir")]
    num_pls_per_dir: u32,

    #[serde(default = "get_default_num_field_stores_per_dir")]
    num_stores_per_dir: u32,

    #[serde(default = "get_default_with_positions")]
    with_positions: bool,
}

impl Default for MorselsIndexingConfig {
    fn default() -> Self {
        MorselsIndexingConfig {
            num_threads: get_default_num_threads(),
            num_docs_per_block: get_default_num_docs_per_block(),
            pl_cache_threshold: get_default_pl_cache_threshold(),
            exclude: get_default_exclude_patterns(),
            loader_configs: get_default_loader_configs(),
            num_pls_per_dir: get_default_num_pls_per_dir(),
            num_stores_per_dir: get_default_num_field_stores_per_dir(),
            with_positions: get_default_with_positions(),
        }
    }
}

impl MorselsIndexingConfig {
    pub fn get_loaders_from_config(&self) -> Vec<Box<dyn Loader>> {
        let mut loaders: Vec<Box<dyn Loader>> = Vec::new();
    
        for (key, value) in self.loader_configs.clone() {
            match &key[..] {
                "HtmlLoader" => loaders.push(HtmlLoader::get_new_html_loader(value)),
                "CsvLoader" => loaders.push(CsvLoader::get_new_csv_loader(value)),
                "JsonLoader" => loaders.push(JsonLoader::get_new_json_loader(value)),
                _ => panic!("Unknown loader type encountered in config")
            }
        }
    
        loaders
    }

    pub fn get_excludes_from_config(&self) -> Vec<Pattern> {
        self.exclude.iter().map(|pat_str| Pattern::new(pat_str).expect("Invalid exclude glob pattern!")).collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct MorselsConfig {
    #[serde(default)]
    fields_config: FieldsConfig,
    #[serde(default)]
    language: MorselsLanguageConfig,
    #[serde(default)]
    pub indexing_config: MorselsIndexingConfig,
}

// Separate struct to support serializing for --init option but not output config
#[derive(Serialize)]
struct MorselsIndexingOutputConfig {
    loader_configs: FxHashMap<String, Box<dyn Loader>>,
    pl_names_to_cache: Vec<u32>,
    num_pls_per_dir: u32,
    num_stores_per_dir: u32,
    with_positions: bool,
}

#[derive(Serialize)]
pub struct MorselsOutputConfig<'a> {
    indexing_config: MorselsIndexingOutputConfig,
    language: &'a MorselsLanguageConfig,
    field_infos: &'a FieldInfos,
}

impl Default for MorselsConfig {
    fn default() -> Self {
        MorselsConfig {
            indexing_config: MorselsIndexingConfig::default(),
            language: MorselsLanguageConfig::default(),
            fields_config: FieldsConfig::default(),
        }
    }
}


pub struct Indexer {
    indexing_config: MorselsIndexingConfig,
    doc_id_counter: u32,
    spimi_counter: u32,
    pl_names_to_cache: Vec<u32>,
    field_infos: Arc<FieldInfos>,
    output_folder_path: PathBuf,
    workers: Vec<Worker>,
    loaders: Vec<Box<dyn Loader>>,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: Sender<MainToWorkerMessage>,
    rx_main: Receiver<WorkerToMainMessage>,
    num_workers_writing_blocks: Arc<Mutex<usize>>,
    language_config: MorselsLanguageConfig,
    dictionary: Dictionary,
    is_dynamic: bool,
    start_doc_id: u32,
    dynamic_index_info: DynamicIndexInfo,
}


impl Indexer {
    #[allow(clippy::mutex_atomic)]
    pub fn new(
        output_folder_path: &Path,
        config: MorselsConfig,
        mut is_dynamic: bool,
    ) -> Indexer {
        is_dynamic = is_dynamic
          && if let Ok(meta) = std::fs::metadata(output_folder_path.join(DYNAMIC_INDEX_INFO_FILE_NAME)) { meta.is_file() } else { false };

        let dynamic_index_info = if is_dynamic {
            dynamic_index_info::DynamicIndexInfo::new_from_output_folder(&output_folder_path)
        } else {
            dynamic_index_info::DynamicIndexInfo::empty()
        };
        
        let loaders = config.indexing_config.get_loaders_from_config();

        let dictionary = if is_dynamic {
            let mut dictionary_table_vec: Vec<u8> = Vec::new();
            let mut dictionary_string_vec: Vec<u8> = Vec::new();
            File::open(output_folder_path.join("dictionaryTable")).unwrap().read_to_end(&mut dictionary_table_vec).unwrap();
            File::open(output_folder_path.join("dictionaryString")).unwrap().read_to_end(&mut dictionary_string_vec).unwrap();

            morsels_common::dictionary::setup_dictionary(dictionary_table_vec, dictionary_string_vec, 0, false)
        } else {
            // Not needed, don't load / decode it
            Dictionary {
                term_infos: FxHashMap::default(),
                trigrams: FxHashMap::default(),
            }
        };

        let field_infos = {
            let mut field_infos_by_name: FxHashMap<String, FieldInfo> = FxHashMap::default();
            for field_config in config.fields_config.fields {
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

            // Assign field ids according to weight

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

            Arc::new(FieldInfos::init(field_infos_by_name, config.fields_config.field_store_block_size, output_folder_path))
        };
        
        let doc_infos = Arc::from(Mutex::from(
            if is_dynamic {
                let mut doc_infos_vec: Vec<u8> = Vec::new();
                File::open(output_folder_path.join("docInfo")).unwrap().read_to_end(&mut doc_infos_vec).unwrap();

                DocInfos::from_search_docinfo(doc_infos_vec, field_infos.num_scored_fields)
            } else {
                DocInfos::init_doc_infos(field_infos.num_scored_fields)
            }
        ));

        let doc_id_counter = doc_infos.lock().unwrap().doc_lengths.len() as u32;
        let start_doc_id = doc_id_counter;

        // Construct worker threads
        let (tx_worker, rx_main) : (Sender<WorkerToMainMessage>, Receiver<WorkerToMainMessage>) = crossbeam::bounded(config.indexing_config.num_threads);
        let (tx_main, rx_worker) : (Sender<MainToWorkerMessage>, Receiver<MainToWorkerMessage>) = crossbeam::bounded(config.indexing_config.num_threads);

        let expected_num_docs_per_thread = (
            config.indexing_config.num_docs_per_block / (config.indexing_config.num_threads as u32) * 2
        ) as usize;
        let num_threads = config.indexing_config.num_threads;

        let num_workers_writing_blocks = Arc::from(Mutex::from(0));

        let tokenizer = Indexer::resolve_tokenizer(&config.language);

        let mut workers = Vec::with_capacity(num_threads);
        let num_stores_per_dir = config.indexing_config.num_stores_per_dir;
        let with_positions = config.indexing_config.with_positions;
        for i in 0..num_threads {
            let tx_worker_clone = tx_worker.clone();
            let rx_worker_clone = rx_worker.clone();
            let tokenize_clone = Arc::clone(&tokenizer);
            let field_info_clone = Arc::clone(&field_infos);
            let num_workers_writing_blocks_clone = Arc::clone(&num_workers_writing_blocks);

            workers.push(Worker {
                id: i as usize,
                join_handle: std::thread::spawn(move ||
                    worker::worker(
                        i as usize,
                        tx_worker_clone,
                        rx_worker_clone,
                        tokenize_clone,
                        field_info_clone,
                        num_stores_per_dir,
                        with_positions,
                        expected_num_docs_per_thread,
                        num_workers_writing_blocks_clone,
                        is_dynamic,
                    )),
            });
        }

        Indexer {
            indexing_config: config.indexing_config,
            doc_id_counter,
            spimi_counter: 0,
            pl_names_to_cache: Vec::new(),
            field_infos,
            output_folder_path: output_folder_path.to_path_buf(),
            workers,
            loaders,
            doc_infos,
            tx_main,
            rx_main,
            num_workers_writing_blocks,
            language_config: config.language,
            dictionary,
            is_dynamic,
            start_doc_id,
            dynamic_index_info,
        }
    }

    fn resolve_tokenizer(language_config: &MorselsLanguageConfig) -> Arc<dyn Tokenizer + Send + Sync> {
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

    fn block_number(&self) -> u32 {
        ((self.doc_id_counter - self.start_doc_id) as f64 / (self.indexing_config.num_docs_per_block as f64)).ceil() as u32
    }

    pub fn index_file(&mut self, input_folder_path_clone: &Path, path: &Path, relative_path: &Path) {
        let timestamp = if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                modified.duration_since(UNIX_EPOCH).unwrap().as_millis()
            } else {
                /*
                 Use program execution time if metadata unavailable.
                 This results in the path always being updated.
                */
                *CURRENT_MILLIS
            }
        } else {
            *CURRENT_MILLIS
        };

        if !self.dynamic_index_info.update_path_if_modified(relative_path, timestamp) && self.is_dynamic {
            return;
        }

        for loader in self.loaders.iter() {
            if let Some(loader_results) = loader.try_index_file(input_folder_path_clone, path, relative_path) {
                for loader_result in loader_results {
                    self.tx_main.send(MainToWorkerMessage::Index {
                        doc_id: self.doc_id_counter,
                        loader_result,
                    }).expect("Failed to send work message to worker!");

                    self.dynamic_index_info.add_doc_to_path(relative_path, self.doc_id_counter);
                
                    self.doc_id_counter += 1;
                    self.spimi_counter += 1;
            
                    if self.spimi_counter == self.indexing_config.num_docs_per_block {
                        let block_number = self.block_number();
                        Indexer::write_block(
                            &self.num_workers_writing_blocks,
                            self.indexing_config.num_threads,
                            &mut self.tx_main,
                            &mut self.rx_main,
                            PathBuf::from(&self.output_folder_path),
                            block_number,
                            self.spimi_counter,
                            self.doc_id_counter - self.spimi_counter,
                            &self.doc_infos,
                        );
                        self.spimi_counter = 0;
                    }
                }
                break;
            }
        }
    }

    pub fn write_morsels_source_config(mut config: MorselsConfig, config_file_path: &Path) {
        config.indexing_config.loader_configs = config.indexing_config.get_loaders_from_config()
            .into_iter()
            .map(|loader| (loader.get_name(), serde_json::to_value(loader).unwrap()))
            .collect();

        File::create(config_file_path)
            .unwrap()
            .write_all(
                serde_json::to_string_pretty(&config)
                    .expect("Failed to serialize morsels config for --init!")
                    .as_bytes()
            )
            .unwrap();
    }

    fn write_morsels_config(&mut self) {
        let serialized = serde_json::to_string(&MorselsOutputConfig {
            indexing_config: MorselsIndexingOutputConfig {
                loader_configs: std::mem::take(&mut self.loaders).into_iter().map(|loader| (loader.get_name(), loader)).collect(),
                pl_names_to_cache: std::mem::take(&mut self.pl_names_to_cache),
                num_pls_per_dir: self.indexing_config.num_pls_per_dir,
                num_stores_per_dir: self.indexing_config.num_stores_per_dir,
                with_positions: self.indexing_config.with_positions,
            },
            language: &self.language_config,
            field_infos: &self.field_infos,
        }).unwrap();

        File::create(self.output_folder_path.join("_morsels_config.json"))
            .unwrap()
            .write_all(serialized.as_bytes())
            .unwrap();
    }

    pub fn finish_writing_docs(mut self, instant: Option<Instant>) {
        if self.spimi_counter != 0 && self.spimi_counter != self.indexing_config.num_docs_per_block {
            println!("Writing last spimi block");
            let block_number = self.block_number();
            Indexer::write_block(
                &self.num_workers_writing_blocks,
                self.indexing_config.num_threads,
                &mut self.tx_main,
                &mut self.rx_main,
                PathBuf::from(&self.output_folder_path),
                block_number,
                self.spimi_counter,
                self.doc_id_counter - self.spimi_counter,
                &self.doc_infos,
            );
            self.spimi_counter = 0;
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
        if self.is_dynamic {
            self.dynamic_index_info.delete_unencountered_paths();

            spimireader::modify_blocks(
                self.doc_id_counter,
                num_blocks,
                &mut self.indexing_config,
                &mut self.pl_names_to_cache,
                std::mem::take(&mut self.doc_infos),
                &self.tx_main,
                &self.output_folder_path,
                &mut self.dictionary,
                &mut self.dynamic_index_info,
            );
        } else {
            spimireader::merge_blocks(
                self.doc_id_counter,
                num_blocks,
                &mut self.indexing_config,
                &mut self.pl_names_to_cache,
                std::mem::take(&mut self.doc_infos),
                &self.tx_main,
                &self.output_folder_path,
                &mut self.dynamic_index_info,
            );
        }

        self.write_morsels_config();

        self.dynamic_index_info.write(&self.output_folder_path, self.doc_id_counter);

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
