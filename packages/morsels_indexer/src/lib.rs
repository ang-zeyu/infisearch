mod docinfo;
mod incremental_info;
pub mod fieldinfo;
pub mod loader;
mod spimireader;
mod spimiwriter;
mod utils;
mod worker;

use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::{Read, Write, BufWriter};
use std::iter::FromIterator;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use morsels_common::tokenize::IndexerTokenizer;
use morsels_common::{MorselsLanguageConfig, BITMAP_DOCINFO_DICT_TABLE_FILE, BitmapDocinfoDicttableReader};
use morsels_lang_ascii::ascii;
use morsels_lang_latin::latin;
use morsels_lang_chinese::chinese;

use crate::docinfo::DocInfos;
use crate::incremental_info::IncrementalIndexInfo;
use crate::fieldinfo::FieldInfo;
use crate::fieldinfo::FieldInfos;
use crate::fieldinfo::FieldsConfig;
use crate::loader::csv::CsvLoader;
use crate::loader::html::HtmlLoader;
use crate::loader::json::JsonLoader;
use crate::loader::txt::TxtLoader;
use crate::loader::Loader;
use crate::worker::miner::WorkerMiner;
use crate::worker::{MainToWorkerMessage, Worker, WorkerToMainMessage};

use crossbeam::channel::{self, Receiver, Sender};
use glob::Pattern;
use normalize_line_endings::normalized;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[macro_use]
extern crate lazy_static;

pub const MORSELS_VERSION: &str = env!("CARGO_PKG_VERSION");

fn get_default_num_threads() -> usize {
    std::cmp::max(num_cpus::get_physical() - 1, 1)
}

fn get_default_pl_limit() -> u32 {
    5242880
}

fn get_default_num_docs_per_block() -> u32 {
    1000
}

fn get_default_pl_cache_threshold() -> u32 {
    0
}

fn get_default_exclude_patterns() -> Vec<String> {
    vec!["morsels_config.json".to_owned()]
}

fn get_default_loader_configs() -> FxHashMap<String, serde_json::Value> {
    let mut configs = FxHashMap::default();

    configs.insert("HtmlLoader".to_owned(), serde_json::json!({}));

    configs
}

fn get_default_num_pls_per_dir() -> u32 {
    1000
}

fn get_default_with_positions() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct MorselsIndexingConfig {
    #[serde(default = "get_default_num_threads", skip_serializing)]
    num_threads: usize,

    #[serde(default = "get_default_num_docs_per_block")]
    num_docs_per_block: u32,

    #[serde(default = "get_default_pl_limit")]
    pl_limit: u32,

    #[serde(default = "get_default_pl_cache_threshold")]
    pl_cache_threshold: u32,

    #[serde(default = "get_default_exclude_patterns")]
    exclude: Vec<String>,

    #[serde(default = "get_default_loader_configs")]
    loader_configs: FxHashMap<String, serde_json::Value>,

    #[serde(default = "get_default_num_pls_per_dir")]
    num_pls_per_dir: u32,

    #[serde(default = "get_default_with_positions")]
    with_positions: bool,
}

impl Default for MorselsIndexingConfig {
    fn default() -> Self {
        MorselsIndexingConfig {
            num_threads: get_default_num_threads(),
            num_docs_per_block: get_default_num_docs_per_block(),
            pl_limit: get_default_pl_limit(),
            pl_cache_threshold: get_default_pl_cache_threshold(),
            exclude: get_default_exclude_patterns(),
            loader_configs: get_default_loader_configs(),
            num_pls_per_dir: get_default_num_pls_per_dir(),
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
                "TxtLoader" => loaders.push(TxtLoader::get_new_txt_loader(value)),
                _ => panic!("Unknown loader type encountered in config"),
            }
        }

        loaders
    }

    pub fn get_excludes_from_config(&self) -> Vec<Pattern> {
        self.exclude
            .iter()
            .map(|pat_str| Pattern::new(pat_str).expect("Invalid exclude glob pattern!"))
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct MorselsConfig {
    #[serde(default)]
    fields_config: FieldsConfig,
    #[serde(default)]
    lang_config: MorselsLanguageConfig,
    #[serde(default)]
    pub indexing_config: MorselsIndexingConfig,
    #[serde(skip)]
    pub raw_config: String,
}

// Separate struct to support serializing for --config-init option but not output config
#[derive(Serialize)]
struct MorselsIndexingOutputConfig {
    loader_configs: FxHashMap<String, Box<dyn Loader>>,
    pl_names_to_cache: Vec<u32>,
    num_docs_per_block: u32,
    num_pls_per_dir: u32,
    with_positions: bool,
}

#[derive(Serialize)]
pub struct MorselsOutputConfig<'a> {
    ver: &'static str,
    last_doc_id: u32,
    indexing_config: MorselsIndexingOutputConfig,
    lang_config: &'a MorselsLanguageConfig,
    cache_all_field_stores: bool,
    field_infos: &'a FieldInfos,
}

impl Default for MorselsConfig {
    fn default() -> Self {
        MorselsConfig {
            indexing_config: MorselsIndexingConfig::default(),
            lang_config: MorselsLanguageConfig::default(),
            fields_config: FieldsConfig::default(),
            raw_config: "".to_owned(),
        }
    }
}

pub struct Indexer {
    indexing_config: Arc<MorselsIndexingConfig>,
    doc_id_counter: u32,
    spimi_counter: u32,
    cache_all_field_stores: bool,
    field_infos: Arc<FieldInfos>,
    output_folder_path: PathBuf,
    doc_miner: WorkerMiner,
    workers: Vec<Worker>,
    loaders: Vec<Box<dyn Loader>>,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: Sender<MainToWorkerMessage>,
    rx_main: Receiver<WorkerToMainMessage>,
    rx_worker: Receiver<MainToWorkerMessage>,
    num_workers_writing_blocks: Arc<Mutex<usize>>,
    lang_config: MorselsLanguageConfig,
    is_incremental: bool,
    start_doc_id: u32,
    start_block_number: u32,
    incremental_info: IncrementalIndexInfo,
}

impl Indexer {
    #[allow(clippy::mutex_atomic)]
    pub fn new(
        output_folder_path: &Path,
        config: MorselsConfig,
        mut is_incremental: bool,
        use_content_hash: bool,
        preserve_output_folder: bool,
    ) -> Indexer {
        fs::create_dir_all(output_folder_path).expect("could not create output directory!");

        let raw_config_normalised = &String::from_iter(normalized(config.raw_config.chars()));

        let bitmap_docinfo_dicttable_path = output_folder_path.join(BITMAP_DOCINFO_DICT_TABLE_FILE);
        let mut bitmap_docinfo_dicttable_rdr = if let Ok(mut file) = File::open(bitmap_docinfo_dicttable_path) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            Some(BitmapDocinfoDicttableReader { buf, pos: 0 })
        } else {
            None
        };

        let mut incremental_info = IncrementalIndexInfo::new_from_output_folder(
            output_folder_path,
            raw_config_normalised,
            &mut is_incremental,
            use_content_hash,
            bitmap_docinfo_dicttable_rdr.as_mut(),
        );

        if !is_incremental && !preserve_output_folder {
            if let Ok(read_dir) = fs::read_dir(output_folder_path) {
                for dir_entry in read_dir {
                    if let Err(err) = dir_entry {
                        eprintln!("Failed to clean {}, continuing.", err);
                        continue;
                    }

                    let dir_entry = dir_entry.unwrap();
                    let file_type = dir_entry.file_type();
                    if let Err(err) = file_type {
                        eprintln!("Failed to get file type when cleaning output dir {}, continuing.", err);
                        continue;
                    }

                    let file_type = file_type.unwrap();
                    if file_type.is_file() {
                        if let Err(err) = fs::remove_file(dir_entry.path()) {
                            eprintln!("{}\nFailed to clean {}, continuing.", err, dir_entry.path().to_string_lossy());
                        }
                    } else if file_type.is_dir() {
                        if let Err(err) = fs::remove_dir_all(dir_entry.path()) {
                            eprintln!("{}\nFailed to clean directory {}, continuing.", err, dir_entry.path().to_string_lossy());
                        }
                    }
                }
            } else {
                eprintln!("Failed to read output dir for cleaning, continuing.");
            }
        }

        {
            File::create(output_folder_path.join("old_morsels_config.json"))
                .expect("error creating old config file")
                .write_all(raw_config_normalised.as_bytes())
                .expect("error writing old config");
        }

        let loaders = config.indexing_config.get_loaders_from_config();

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
                    },
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

            Arc::new(FieldInfos::init(
                field_infos_by_name,
                config.fields_config.field_store_block_size,
                config.fields_config.num_stores_per_dir,
                output_folder_path,
            ))
        };


        let doc_infos = Arc::from(Mutex::from(if is_incremental {
            DocInfos::from_search_docinfo(
                bitmap_docinfo_dicttable_rdr.as_mut().expect("missing docinfo metadata file!"),
                field_infos.num_scored_fields,
            )
        } else {
            DocInfos::init_doc_infos(field_infos.num_scored_fields)
        }));

        if is_incremental {
            incremental_info.setup_dictionary(
                output_folder_path,
                bitmap_docinfo_dicttable_rdr.as_mut().expect("missing dicttable metadata file!"),
            );
        }

        let doc_id_counter = doc_infos.lock().unwrap().doc_lengths.len() as u32;

        #[cfg(debug_assertions)]
        println!("previous number of docs {}", doc_id_counter);

        let spimi_counter = doc_id_counter % config.indexing_config.num_docs_per_block;

        // Construct worker threads
        let (tx_worker, rx_main): (
            Sender<WorkerToMainMessage>, Receiver<WorkerToMainMessage>
        ) = channel::bounded(config.indexing_config.num_threads);
        let (tx_main, rx_worker): (
            Sender<MainToWorkerMessage>, Receiver<MainToWorkerMessage>
        ) = channel::bounded(32); // TODO may be a little arbitrary

        let expected_num_docs_per_thread =
            (config.indexing_config.num_docs_per_block / (config.indexing_config.num_threads as u32) * 2) as usize;
        let num_threads = config.indexing_config.num_threads;

        let num_workers_writing_blocks = Arc::from(Mutex::from(0));

        let tokenizer = Indexer::resolve_tokenizer(&config.lang_config);

        let indexing_config = Arc::from(config.indexing_config);

        let mut workers = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let id = i;
            let tx_worker_clone = tx_worker.clone();
            let rx_worker_clone = rx_worker.clone();
            let tokenize_clone = Arc::clone(&tokenizer);
            let field_info_clone = Arc::clone(&field_infos);
            let indexing_config_clone = Arc::clone(&indexing_config);
            let num_workers_writing_blocks_clone = Arc::clone(&num_workers_writing_blocks);

            workers.push(Worker {
                id,
                join_handle: std::thread::spawn(move || {
                    worker::worker(
                        id,
                        tx_worker_clone,
                        rx_worker_clone,
                        tokenize_clone,
                        field_info_clone,
                        indexing_config_clone,
                        expected_num_docs_per_thread,
                        num_workers_writing_blocks_clone,
                    )
                }),
            });
        }

        let doc_miner = WorkerMiner::new(
            &field_infos,
            indexing_config.with_positions,
            expected_num_docs_per_thread,
            &tokenizer,
            #[cfg(debug_assertions)]
            0,
        );

        let mut indexer = Indexer {
            indexing_config,
            doc_id_counter,
            spimi_counter,
            cache_all_field_stores: config.fields_config.cache_all_field_stores,
            field_infos,
            output_folder_path: output_folder_path.to_path_buf(),
            doc_miner,
            workers,
            loaders,
            doc_infos,
            tx_main,
            rx_main,
            rx_worker,
            num_workers_writing_blocks,
            lang_config: config.lang_config,
            is_incremental,
            start_doc_id: doc_id_counter,
            start_block_number: 0,
            incremental_info,
        };
        indexer.start_block_number = indexer.block_number();

        indexer
    }

    fn resolve_tokenizer(lang_config: &MorselsLanguageConfig) -> Arc<dyn IndexerTokenizer + Send + Sync> {
        match lang_config.lang.as_str() {
            "ascii" => {
                if let Some(options) = lang_config.options.as_ref() {
                    Arc::new(ascii::new_with_options(serde_json::from_value(options.clone()).unwrap(), false))
                } else {
                    Arc::new(ascii::Tokenizer::default())
                }
            }
            "latin" => {
                if let Some(options) = lang_config.options.as_ref() {
                    Arc::new(latin::new_with_options(serde_json::from_value(options.clone()).unwrap(), false))
                } else {
                    Arc::new(latin::Tokenizer::default())
                }
            }
            "chinese" => {
                if let Some(options) = lang_config.options.as_ref() {
                    Arc::new(chinese::new_with_options(serde_json::from_value(options.clone()).unwrap(), false))
                } else {
                    Arc::new(chinese::Tokenizer::default())
                }
            }
            _ => {
                panic!("Unsupported language {}", lang_config.lang)
            }
        }
    }

    fn block_number(&self) -> u32 {
        ((self.doc_id_counter as f64) / (self.indexing_config.num_docs_per_block as f64)).floor() as u32
    }

    pub fn index_file(&mut self, input_folder_path_clone: &Path, path: &Path, relative_path: &Path) {
        let relative_path_lossy;
        let external_id = if let Some(relative_path) = relative_path.to_str() {
            relative_path
        } else {
            relative_path_lossy = relative_path.to_string_lossy().into_owned();
            &relative_path_lossy
        };

        for loader in self.loaders.iter() {
            if let Some(loader_results) = loader.try_index_file(input_folder_path_clone, path, relative_path)
            {
                let is_not_modified = self.incremental_info.set_file(external_id, path);
                if is_not_modified && self.is_incremental {
                    return;
                }

                for loader_result in loader_results {
                    self.tx_main.send(MainToWorkerMessage::Index {
                        doc_id: self.doc_id_counter,
                        loader_result,
                    }).expect("Failed to send index msg to worker!");


                    Self::try_index_doc(&mut self.doc_miner, &self.rx_worker, 30); // TODO 30 a little arbitrary?

                    self.incremental_info.add_doc_to_file(external_id, self.doc_id_counter);

                    self.doc_id_counter += 1;
                    self.spimi_counter += 1;

                    if self.spimi_counter == self.indexing_config.num_docs_per_block {
                        Self::try_index_doc(&mut self.doc_miner, &self.rx_worker, 0);

                        let main_thread_block_index_results = self.doc_miner.get_results();
                        let block_number = self.block_number() - 1;

                        self.merge_block(
                            main_thread_block_index_results, block_number, false,
                        );
                        self.spimi_counter = 0;
                    }
                }

                break;
            }
        }
    }

    fn try_index_doc(doc_miner: &mut WorkerMiner, rx_worker: &Receiver<MainToWorkerMessage>, until: usize) {
        while rx_worker.len() > until {
            if let Ok(msg) = rx_worker.try_recv() {
                if let MainToWorkerMessage::Index { doc_id, mut loader_result } = msg {
                    doc_miner.index_doc(doc_id, loader_result.get_field_texts());
                } else {
                    panic!("Unexpected message received @main thread");
                }
            }
        }
    }

    pub fn write_morsels_source_config(mut config: MorselsConfig, config_file_path: &Path) {
        config.indexing_config.loader_configs = config
            .indexing_config
            .get_loaders_from_config()
            .into_iter()
            .map(|loader| (loader.get_name(), serde_json::to_value(loader).unwrap()))
            .collect();

        File::create(config_file_path)
            .unwrap()
            .write_all(
                serde_json::to_string_pretty(&config)
                    .expect("Failed to serialize morsels config for --config-init!")
                    .as_bytes(),
            )
            .unwrap();
    }

    pub fn finish_writing_docs(mut self, instant: Option<Instant>) {
        #[cfg(debug_assertions)]
        println!("@finish_writing_docs");

        let first_block = self.start_block_number;
        let mut last_block = self.block_number();

        if self.spimi_counter != 0 {
            Self::try_index_doc(&mut self.doc_miner, &self.rx_worker, 0);

            #[cfg(debug_assertions)]
            println!("Writing extra last spimi block");

            let main_thread_block_index_results = self.doc_miner.get_results();
            self.merge_block(main_thread_block_index_results, last_block, true);
            self.spimi_counter = 0;
        } else if !self.is_deletion_only_run() {
            last_block -= 1;
        }
        self.wait_on_all_workers();

        #[cfg(debug_assertions)]
        println!(
            "Number of docs: {}, First Block {}, Last Block {}",
            self.doc_id_counter, first_block, last_block,
        );

        if let Some(now) = instant {
            print_time_elapsed(now, "Block indexing done!");
        }

        // Merge spimi blocks
        // Go through all blocks at once
        self.merge_blocks(first_block, last_block);

        self.write_morsels_config();

        self.incremental_info.write_info(&self.output_folder_path);

        if !self.is_deletion_only_run() {
            spimireader::common::cleanup_blocks(first_block, last_block, &self.output_folder_path);
        }

        if let Some(now) = instant {
            print_time_elapsed(now, "Blocks merged!");
        }

        self.terminate_all_workers();
    }

    fn is_deletion_only_run(&self) -> bool {
        self.doc_id_counter == self.start_doc_id
    }

    fn merge_blocks(&mut self, first_block: u32, last_block: u32) {
        let bitmap_docinfo_dicttable_file = self.output_folder_path.join(BITMAP_DOCINFO_DICT_TABLE_FILE);
        let mut bitmap_docinfo_dicttable_writer = BufWriter::new(
            File::create(bitmap_docinfo_dicttable_file).unwrap()
        );

        let num_blocks = last_block - first_block + 1;
        if self.is_incremental {
            self.incremental_info.delete_unencountered_external_ids();
            self.incremental_info.write_invalidation_vec(&mut bitmap_docinfo_dicttable_writer, self.doc_id_counter);

            spimireader::incremental::modify_blocks(
                self.is_deletion_only_run(),
                self.doc_id_counter,
                num_blocks,
                first_block,
                last_block,
                &self.indexing_config,
                &self.field_infos,
                std::mem::take(&mut self.doc_infos),
                &self.tx_main,
                &self.output_folder_path,
                bitmap_docinfo_dicttable_writer,
                &mut self.incremental_info,
            );
        } else {
            self.incremental_info.write_invalidation_vec(&mut bitmap_docinfo_dicttable_writer, self.doc_id_counter);

            spimireader::full::merge_blocks(
                self.doc_id_counter,
                num_blocks,
                first_block,
                last_block,
                &self.indexing_config,
                &self.field_infos,
                std::mem::take(&mut self.doc_infos),
                &self.tx_main,
                &self.output_folder_path,
                bitmap_docinfo_dicttable_writer,
                &mut self.incremental_info,
            );
        }
    }

    fn write_morsels_config(&mut self) {
        let serialized = serde_json::to_string(&MorselsOutputConfig {
            ver: MORSELS_VERSION,
            last_doc_id: self.doc_id_counter,
            indexing_config: MorselsIndexingOutputConfig {
                loader_configs: std::mem::take(&mut self.loaders)
                    .into_iter()
                    .map(|loader| (loader.get_name(), loader))
                    .collect(),
                pl_names_to_cache: self.incremental_info.pl_names_to_cache.clone(),
                num_docs_per_block: self.indexing_config.num_docs_per_block,
                num_pls_per_dir: self.indexing_config.num_pls_per_dir,
                with_positions: self.indexing_config.with_positions,
            },
            lang_config: &self.lang_config,
            cache_all_field_stores: self.cache_all_field_stores,
            field_infos: &self.field_infos,
        })
        .unwrap();

        File::create(self.output_folder_path.join("morsels_config.json"))
            .unwrap()
            .write_all(serialized.as_bytes())
            .unwrap();
    }
}

fn print_time_elapsed(instant: Instant, extra_message: &str) {
    let elapsed = instant.elapsed().as_secs_f64();
    println!("({}) {} mins {} seconds elapsed.", extra_message, (elapsed as u32) / 60, elapsed % 60.0);
}
