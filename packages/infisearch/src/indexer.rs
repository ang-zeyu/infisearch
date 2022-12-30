pub mod input_config;
pub mod output_config;
mod spimi_writer;
mod worker;

use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Instant, UNIX_EPOCH, SystemTime};

use infisearch_common::language::InfiLanguageConfig;
use infisearch_common::METADATA_FILE;
use infisearch_common::tokenize::IndexerTokenizer;
use infisearch_lang_ascii::ascii;
use infisearch_lang_ascii_stemmer::ascii_stemmer;
use infisearch_lang_chinese::chinese;

use crate::dictionary_writer::DictWriter;
use crate::doc_info::DocInfos;
use crate::utils::{fs_utils, time};
use crate::{i_debug, spimi_reader, OLD_SOURCE_CONFIG};
use crate::incremental_info::IncrementalIndexInfo;
use crate::field_info::FieldInfos;
use crate::indexer::input_config::{InfiConfig, InfiIndexingConfig};
use crate::loader::LoaderBoxed;
use crate::worker::miner::WorkerMiner;
use crate::worker::{create_worker, MainToWorkerMessage, Worker, WorkerToMainMessage};

use crossbeam::channel::{self, Receiver, Sender};

pub struct Indexer {
    indexing_config: Arc<InfiIndexingConfig>,
    doc_id_counter: u32,
    spimi_counter: u32,
    cache_all_field_stores: bool,
    field_infos: Arc<FieldInfos>,
    index_ver: String,
    input_folder_path: PathBuf,
    output_folder_path: PathBuf,
    output_folder_path_inner: PathBuf,
    doc_miner: WorkerMiner,
    workers: Vec<Worker>,
    loaders: Arc<Vec<LoaderBoxed>>,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: Sender<MainToWorkerMessage>,
    rx_main: Receiver<WorkerToMainMessage>,
    rx_worker: Receiver<MainToWorkerMessage>,
    num_workers_writing_blocks: Arc<Mutex<usize>>,
    lang_config: InfiLanguageConfig,
    is_incremental: bool,
    start_doc_id: u32,
    start_block_number: u32,
    incremental_info: IncrementalIndexInfo,
    start_instant: Option<Instant>,
}

impl Indexer {
    #[allow(clippy::mutex_atomic)]
    pub fn new(
        input_folder_path: &Path,
        output_folder_path: &Path,
        config: InfiConfig,
        is_incremental: bool,
        use_content_hash: bool,
        preserve_output_folder: bool,
        log_perf: bool,
    ) -> Indexer {
        // -----------------------------------------------------------

        fs::create_dir_all(&output_folder_path).expect("could not create output directory!");

        let index_ver = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() / 100).to_string();
        let output_folder_path_inner = output_folder_path.join(&index_ver); // For cache-busting

        // -----------------------------------------------------------

        // -----------------------------------------------------------
        // Initialise the previously indexed metadata, if any

        let (
            incremental_output_config,
            mut metadata_rdr,
            mut incremental_info,
        ) = IncrementalIndexInfo::new_from_output_folder(
            &output_folder_path_inner,
            output_folder_path,
            &config.json_config,
            is_incremental,
            use_content_hash,
        );
        let is_incremental = incremental_output_config.is_some();

        if !output_folder_path_inner.exists() {
            fs::create_dir(&output_folder_path_inner).expect("could not create inner output directory!");
        }
        
        // -----------------------------------------------------------

        // -----------------------------------------------------------
        // Clean the output folder if running a full index
        if !is_incremental && !preserve_output_folder {
            fs_utils::clean_dir(output_folder_path);
        }
        // -----------------------------------------------------------

        // -----------------------------------------------------------
        // Store the current raw json configuration file, for checking if it changed in the next run

        fs::write(
            output_folder_path.join(OLD_SOURCE_CONFIG),
            serde_json::to_string_pretty(&config.json_config)
                .expect("Failed to serialize current configuration file"),
        )
        .expect(&("Failed to write ".to_owned() + OLD_SOURCE_CONFIG));

        // -----------------------------------------------------------

        // -----------------------------------------------------------
        // Misc

        let loaders: Arc<Vec<LoaderBoxed>> = Arc::new(config.indexing_config.get_loaders_from_config());

        let field_infos = config.fields_config.get_field_infos(
            &output_folder_path_inner, incremental_output_config.as_ref(),
        );

        // ------------------------------
        // Previous index info
        let doc_infos = Arc::from(Mutex::from(DocInfos::init_doc_infos(
            &field_infos,
            metadata_rdr.as_mut(),
        )));

        if is_incremental {
            incremental_info.setup_dictionary(
                metadata_rdr.as_mut().expect("missing dicttable metadata file!"),
            );
        }
        // ------------------------------

        let doc_id_counter = doc_infos.lock()
            .expect("Unexpected concurrent holding of doc_infos mutex")
            .doc_infos.len() as u32;

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
            let output_folder_path_inner_clone = output_folder_path_inner.to_path_buf();
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
                        output_folder_path_inner_clone,
                        loaders_clone,
                        log_perf,
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
            index_ver,
            input_folder_path: input_folder_path.to_path_buf(),
            output_folder_path: output_folder_path.to_path_buf(),
            output_folder_path_inner,
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
            start_instant: if log_perf { Some(Instant::now()) } else { None },
        };
        indexer.start_block_number = indexer.block_number();

        indexer
    }

    fn resolve_tokenizer(lang_config: &InfiLanguageConfig) -> Arc<dyn IndexerTokenizer + Send + Sync> {
        match lang_config.lang.as_str() {
            "ascii" => Arc::new(ascii::new_with_options(lang_config)),
            "ascii_stemmer" => Arc::new(ascii_stemmer::new_with_options(lang_config)),
            "chinese" => Arc::new(chinese::new_with_options(lang_config)),
            _ => panic!("Unsupported language {}", lang_config.lang),
        }
    }

    fn block_number(&self) -> u32 {
        ((self.doc_id_counter as f64) / (self.indexing_config.num_docs_per_block as f64)).floor() as u32
    }

    pub fn index_file(&mut self, absolute_path: &Path, relative_path: &Path) {
        if self.indexing_config.is_excluded(relative_path) {
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

    pub fn finish_writing_docs(mut self) -> u32 {
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
        } else if !self.has_docs_added() {
            last_block -= 1;
        }
        self.wait_on_all_workers();

        i_debug!(
            "Number of docs: {}, First Block {}, Last Block {}",
            self.doc_id_counter, first_block, last_block,
        );

        time::print_time_elapsed(&self.start_instant, "Block indexing done!");

        // N-way merge of spimi blocks
        let enums_ev_strs = self.merge_blocks(first_block, last_block, self.start_instant.is_some());

        self.incremental_info.write_info(&self.input_folder_path, &self.output_folder_path);

        spimi_reader::common::cleanup_blocks(first_block, last_block, &self.output_folder_path_inner);

        time::print_time_elapsed(&self.start_instant, "Blocks merged!");

        // mem::replace/take to circumvent partial move, TODO find a cleaner solution
        Self::terminate_all_workers(
            std::mem::replace(&mut self.tx_main, channel::bounded(0).0),
            std::mem::take(&mut self.workers),
        );

        let total_docs = self.doc_id_counter - self.incremental_info.num_deleted_docs;

        // Config needs to be written after workers are joined, as it calls Arc::try_unwrap.
        output_config::write_output_config(self, enums_ev_strs);

        total_docs
    }

    fn has_docs_added(&self) -> bool {
        self.doc_id_counter == self.start_doc_id
    }

    fn flush_doc_infos(&mut self, num_docs: f64) -> (Vec<u8>, Vec<Vec<String>>) {
        let mut doc_infos_unwrapped_inner = Arc::try_unwrap(std::mem::take(&mut self.doc_infos))
            .expect("No thread should be holding doc infos arc when merging blocks")
            .into_inner()
            .expect("No thread should be holding doc infos mutex when merging blocks");

        doc_infos_unwrapped_inner.finalize_and_flush(
            num_docs as u32,
            &self.field_infos,
            &mut self.incremental_info,
        )
    }

    pub fn flush_metadata(
        &self,
        invalidation_vec_ser: Vec<u8>,
        doc_infos_ser: Vec<u8>,
        dict_writer: DictWriter,
        log_sizes: bool,
    ) {
        let metadata_file = self.output_folder_path_inner.join(METADATA_FILE);
        let mut metadata_writer = BufWriter::new(
            File::create(metadata_file).unwrap()
        );

        let (dict_table_ser, dict_string_ser) = dict_writer.flush();
        let dict_table_ser: &[u8] = dict_table_ser.as_raw_slice();
    
        /*
         Store the dictionary string first for better gzip compression,
         followed by the dict table for locality.

         The invalidation vec and docinfo needs to be read first however,
         so store 3 u32 offsets totalling 12 bytes:
         - dictionary table
         - invalidation vec
         - docinfo
         */

        if log_sizes {
            println!("Metadata lengths:");
            println!("  Dictionary string: {}", dict_string_ser.len());
            println!("  Dictionary table: {}", dict_table_ser.len());
            println!("  Invalidation Vec: {}", invalidation_vec_ser.len());
            println!("  Doc Infos: {}", doc_infos_ser.len());
        }

        let dict_table_offset = 12 + dict_string_ser.len() as u32;
        let invalidation_vec_offset = dict_table_offset + dict_table_ser.len() as u32;
        let doc_infos_offset = invalidation_vec_offset + invalidation_vec_ser.len() as u32;

        metadata_writer.write_all(&dict_table_offset.to_le_bytes()).unwrap();
        metadata_writer.write_all(&invalidation_vec_offset.to_le_bytes()).unwrap();
        metadata_writer.write_all(&doc_infos_offset.to_le_bytes()).unwrap();

        metadata_writer.write_all(&dict_string_ser).unwrap();
        metadata_writer.write_all(&dict_table_ser).unwrap();
        metadata_writer.write_all(&invalidation_vec_ser).unwrap();
        metadata_writer.write_all(&doc_infos_ser).unwrap();

        metadata_writer.flush().expect("Failed to flush metadata.json");
    }

    fn merge_blocks(&mut self, first_block: u32, last_block: u32, log_metadata_sizes: bool) -> Vec<Vec<String>> {
        let num_blocks = last_block - first_block + 1;

        if self.is_incremental {
            self.incremental_info.delete_unencountered_external_ids();
            let invalidation_vec_ser = self.incremental_info.write_invalidation_vec(self.doc_id_counter);
            let (doc_infos_ser, enums_ev_strs) = self.flush_doc_infos(
                (self.doc_id_counter - self.incremental_info.num_deleted_docs) as f64,
            );

            let dict_writer = spimi_reader::incremental::modify_blocks(
                self.has_docs_added(),
                num_blocks,
                first_block,
                last_block,
                &self.indexing_config,
                &self.field_infos,
                &self.tx_main,
                &self.output_folder_path_inner,
                &mut self.incremental_info,
            );
            
            self.flush_metadata(
                invalidation_vec_ser,
                doc_infos_ser,
                dict_writer,
                log_metadata_sizes,
            );

            enums_ev_strs
        } else {
            let invalidation_vec_ser = self.incremental_info.write_invalidation_vec(self.doc_id_counter);
            let (doc_infos_ser, enums_ev_strs) = self.flush_doc_infos(self.doc_id_counter as f64);

            let dict_writer = spimi_reader::full::merge_blocks(
                self.has_docs_added(),
                num_blocks,
                first_block,
                last_block,
                &self.indexing_config,
                &self.field_infos,
                &self.tx_main,
                &self.output_folder_path_inner,
                &mut self.incremental_info,
            );

            self.flush_metadata(
                invalidation_vec_ser,
                doc_infos_ser,
                dict_writer,
                log_metadata_sizes,
            );

            enums_ev_strs
        }
    }
}
