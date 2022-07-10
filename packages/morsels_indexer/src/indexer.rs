pub mod input_config;
pub mod output_config;
mod spimiwriter;
mod worker;

use std::fs::{self, File};
use std::io::{Read, Write, BufWriter};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use log::{info, warn};
use morsels_common::tokenize::IndexerTokenizer;
use morsels_common::{MorselsLanguageConfig, BITMAP_DOCINFO_DICT_TABLE_FILE, BitmapDocinfoDicttableReader};
use morsels_lang_ascii::ascii;
use morsels_lang_latin::latin;
use morsels_lang_chinese::chinese;

use crate::docinfo::DocInfos;
use crate::{i_debug, spimireader};
use crate::incremental_info::IncrementalIndexInfo;
use crate::fieldinfo::FieldInfos;
use crate::indexer::input_config::{MorselsConfig, MorselsIndexingConfig};
use crate::loader::LoaderBoxed;
use crate::worker::miner::WorkerMiner;
use crate::worker::{create_worker, MainToWorkerMessage, Worker, WorkerToMainMessage};

use crossbeam::channel::{self, Receiver, Sender};

pub struct Indexer {
    indexing_config: Arc<MorselsIndexingConfig>,
    doc_id_counter: u32,
    spimi_counter: u32,
    cache_all_field_stores: bool,
    field_infos: Arc<FieldInfos>,
    input_folder_path: PathBuf,
    output_folder_path: PathBuf,
    doc_miner: WorkerMiner,
    workers: Vec<Worker>,
    loaders: Arc<Vec<LoaderBoxed>>,
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
        input_folder_path: &Path,
        output_folder_path: &Path,
        config: MorselsConfig,
        mut is_incremental: bool,
        use_content_hash: bool,
        preserve_output_folder: bool,
    ) -> Indexer {
        fs::create_dir_all(output_folder_path).expect("could not create output directory!");

        // -----------------------------------------------------------
        // Initialise the previously indexed metadata, if any

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
            &config.json_config,
            &mut is_incremental,
            use_content_hash,
            bitmap_docinfo_dicttable_rdr.as_mut(),
        );
        // -----------------------------------------------------------

        // -----------------------------------------------------------
        // Clean the output folder if running a full index
        if !is_incremental && !preserve_output_folder {
            if let Ok(read_dir) = fs::read_dir(output_folder_path) {
                for dir_entry in read_dir {
                    if let Err(err) = dir_entry {
                        warn!("Failed to clean {}, continuing.", err);
                        continue;
                    }

                    let dir_entry = dir_entry.unwrap();
                    let file_type = dir_entry.file_type();
                    if let Err(err) = file_type {
                        warn!("Failed to get file type when cleaning output dir {}, continuing.", err);
                        continue;
                    }

                    let file_type = file_type.unwrap();
                    if file_type.is_file() {
                        if let Err(err) = fs::remove_file(dir_entry.path()) {
                            warn!("{}\nFailed to clean {}, continuing.", err, dir_entry.path().to_string_lossy());
                        }
                    } else if file_type.is_dir() {
                        if let Err(err) = fs::remove_dir_all(dir_entry.path()) {
                            warn!("{}\nFailed to clean directory {}, continuing.", err, dir_entry.path().to_string_lossy());
                        }
                    }
                }
            } else {
                warn!("Failed to read output dir for cleaning, continuing.");
            }
        }
        // -----------------------------------------------------------

        // -----------------------------------------------------------
        // Store the current raw json configuration file, for checking if it changed in the next run

        fs::write(
            output_folder_path.join("old_morsels_config.json"),
            serde_json::to_string_pretty(&config.json_config)
                .expect("Failed to serialize current configuration file"),
        )
        .expect("Failed to write old_morsels_config.json");

        // -----------------------------------------------------------

        // -----------------------------------------------------------
        // Misc

        let loaders: Arc<Vec<LoaderBoxed>> = Arc::new(config.indexing_config.get_loaders_from_config());

        let field_infos = config.fields_config.get_field_infos(output_folder_path);

        // ------------------------------
        // Previous index info
        let doc_infos = Arc::from(Mutex::from(DocInfos::init_doc_infos(
            is_incremental,
            field_infos.num_scored_fields,
            bitmap_docinfo_dicttable_rdr.as_mut(),
        )));

        if is_incremental {
            incremental_info.setup_dictionary(
                output_folder_path,
                bitmap_docinfo_dicttable_rdr.as_mut().expect("missing dicttable metadata file!"),
            );
        }
        // ------------------------------

        let doc_id_counter = doc_infos.lock()
            .expect("Unexpected concurrent holding of doc_infos mutex")
            .doc_lengths.len() as u32;

        i_debug!("Previous number of docs {}", doc_id_counter);

        let spimi_counter = doc_id_counter % config.indexing_config.num_docs_per_block;

        let tokenizer = Indexer::resolve_tokenizer(&config.lang_config);

        let indexing_config = Arc::from(config.indexing_config);

        // -----------------------------------------------------------
        // Construct worker threads
        let (tx_worker, rx_main): (
            Sender<WorkerToMainMessage>, Receiver<WorkerToMainMessage>
        ) = channel::bounded(indexing_config.num_threads);
        let (tx_main, rx_worker): (
            Sender<MainToWorkerMessage>, Receiver<MainToWorkerMessage>
        ) = channel::bounded(32); // TODO may be a little arbitrary

        let expected_num_docs_per_thread =
            (indexing_config.num_docs_per_block / (indexing_config.num_threads as u32) * 2) as usize;
        let num_threads = indexing_config.num_threads;

        let num_workers_writing_blocks = Arc::from(Mutex::from(0));

        let mut workers = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let id = i;
            let tx_worker_clone = tx_worker.clone();
            let rx_worker_clone = rx_worker.clone();
            let tokenize_clone = Arc::clone(&tokenizer);
            let field_info_clone = Arc::clone(&field_infos);
            let indexing_config_clone = Arc::clone(&indexing_config);
            let num_workers_writing_blocks_clone = Arc::clone(&num_workers_writing_blocks);
            let input_folder_path_clone = input_folder_path.to_path_buf();
            let output_folder_path_clone = output_folder_path.to_path_buf();
            let loaders_clone = Arc::clone(&loaders);

            workers.push(Worker {
                id,
                join_handle: std::thread::spawn(move || {
                    create_worker(
                        id,
                        tx_worker_clone,
                        rx_worker_clone,
                        tokenize_clone,
                        field_info_clone,
                        indexing_config_clone,
                        expected_num_docs_per_thread,
                        num_workers_writing_blocks_clone,
                        input_folder_path_clone,
                        output_folder_path_clone,
                        loaders_clone,
                    )
                }),
            });
        }
        // -----------------------------------------------------------

        let doc_miner = WorkerMiner::new(
            &field_infos,
            indexing_config.with_positions,
            expected_num_docs_per_thread,
            &tokenizer,
            input_folder_path.to_path_buf(),
            &loaders,
            #[cfg(debug_assertions)]
            0,
        );

        let mut indexer = Indexer {
            indexing_config,
            doc_id_counter,
            spimi_counter,
            cache_all_field_stores: config.fields_config.cache_all_field_stores,
            field_infos,
            input_folder_path: input_folder_path.to_path_buf(),
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
            "ascii" => Arc::new(ascii::new_with_options(lang_config)),
            "latin" => Arc::new(latin::new_with_options(lang_config)),
            "chinese" => Arc::new(chinese::new_with_options(lang_config)),
            _ => panic!("Unsupported language {}", lang_config.lang),
        }
    }

    fn block_number(&self) -> u32 {
        ((self.doc_id_counter as f64) / (self.indexing_config.num_docs_per_block as f64)).floor() as u32
    }

    pub fn index_file(&mut self, absolute_path: &Path, relative_path: &Path) {
        if let Some(_match) = self.indexing_config.exclude_patterns.iter().find(|pat| pat.matches_path(relative_path)) {
            return;
        }

        let relative_path_lossy;
        let external_id = if let Some(relative_path) = relative_path.to_str() {
            relative_path
        } else {
            relative_path_lossy = relative_path.to_string_lossy().into_owned();
            &relative_path_lossy
        };

        for loader in self.loaders.iter() {
            if let Some(loader_results) = loader.try_index_file(absolute_path, relative_path)
            {
                let is_not_modified = self.incremental_info.set_file(external_id, absolute_path, &self.input_folder_path);
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

                        let secondary_inv_mappings = self.merge_block(
                            main_thread_block_index_results, block_number, false,
                        );
                        self.incremental_info.extend_secondary_inv_mappings(secondary_inv_mappings);

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
                if let MainToWorkerMessage::Index { doc_id, loader_result } = msg {
                    let (field_texts, path) = loader_result.get_field_texts_and_path();
                    doc_miner.index_doc(doc_id, field_texts, path);
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
            .map(|loader| (
                loader.get_name(),
                serde_json::to_value(loader).expect("Failed to convert loader config to serde value")
            ))
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
        i_debug!("@finish_writing_docs");

        let first_block = self.start_block_number;
        let mut last_block = self.block_number();

        if self.spimi_counter != 0 {
            Self::try_index_doc(&mut self.doc_miner, &self.rx_worker, 0);

            i_debug!("Writing extra last spimi block");

            let main_thread_block_index_results = self.doc_miner.get_results();
            let secondary_inv_mappings = self.merge_block(
                main_thread_block_index_results, last_block, true,
            );
            self.incremental_info.extend_secondary_inv_mappings(secondary_inv_mappings);

            self.spimi_counter = 0;
        } else if !self.is_deletion_only_run() {
            last_block -= 1;
        }
        self.wait_on_all_workers();

        i_debug!(
            "Number of docs: {}, First Block {}, Last Block {}",
            self.doc_id_counter, first_block, last_block,
        );

        print_time_elapsed(&instant, "Block indexing done!");

        // N-way merge of spimi blocks
        self.merge_blocks(first_block, last_block);

        self.incremental_info.write_info(&self.input_folder_path, &self.output_folder_path);

        spimireader::common::cleanup_blocks(first_block, last_block, &self.output_folder_path);

        print_time_elapsed(&instant, "Blocks merged!");

        /*
         Circumvent partial move
         TODO find a cleaner solution.

         Config needs to be written after workers are joined, as it calls Arc::try_unwrap.
         */
        let (dummy_tx_main, _): (
            Sender<MainToWorkerMessage>, Receiver<MainToWorkerMessage>
        ) = channel::bounded(0);
        let actual_tx_main = std::mem::replace(&mut self.tx_main, dummy_tx_main);
        let workers = std::mem::take(&mut self.workers);

        Self::terminate_all_workers(actual_tx_main, workers);

        output_config::write_output_config(self);
    }

    fn is_deletion_only_run(&self) -> bool {
        self.doc_id_counter == self.start_doc_id
    }

    fn flush_doc_infos(&mut self, docinfo_dicttable_writer: &mut BufWriter<File>, num_docs: f64) {
        let mut doc_infos_unwrapped_inner = Arc::try_unwrap(std::mem::take(&mut self.doc_infos))
            .expect("No thread should be holding doc infos arc when merging blocks")
            .into_inner()
            .expect("No thread should be holding doc infos mutex when merging blocks");

        doc_infos_unwrapped_inner.finalize_and_flush(
            docinfo_dicttable_writer,
            num_docs as u32, self.field_infos.num_scored_fields,
            &mut self.incremental_info,
        );
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
            self.flush_doc_infos(
                &mut bitmap_docinfo_dicttable_writer,
                (self.doc_id_counter - self.incremental_info.num_deleted_docs) as f64,
            );

            spimireader::incremental::modify_blocks(
                self.is_deletion_only_run(),
                num_blocks,
                first_block,
                last_block,
                &self.indexing_config,
                &self.field_infos,
                &self.tx_main,
                &self.output_folder_path,
                bitmap_docinfo_dicttable_writer,
                &mut self.incremental_info,
            );
        } else {
            self.incremental_info.write_invalidation_vec(&mut bitmap_docinfo_dicttable_writer, self.doc_id_counter);
            self.flush_doc_infos(&mut bitmap_docinfo_dicttable_writer, self.doc_id_counter as f64);

            spimireader::full::merge_blocks(
                num_blocks,
                first_block,
                last_block,
                &self.indexing_config,
                &self.field_infos,
                &self.tx_main,
                &self.output_folder_path,
                bitmap_docinfo_dicttable_writer,
                &mut self.incremental_info,
            );
        }
    }
}

fn print_time_elapsed(instant: &Option<Instant>, extra_message: &str) {
    if let Some(instant) = instant {
        let elapsed = instant.elapsed().as_secs_f64();
        info!("({}) {} mins {} seconds elapsed.", extra_message, (elapsed as u32) / 60, elapsed % 60.0);
    }
}
