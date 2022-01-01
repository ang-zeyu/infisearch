use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::iter::FromIterator;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use normalize_line_endings::normalized;

use morsels_common::dictionary::{self, Dictionary, DICTIONARY_STRING_FILE_NAME, DICTIONARY_TABLE_FILE_NAME};
use morsels_common::{bitmap, BITMAP_FILE_NAME};

use crate::MORSELS_VERSION;

lazy_static! {
    static ref CURRENT_MILLIS: u128 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
}

// Not used for search
static INCREMENTAL_INFO_FILE_NAME: &str = "_incremental_info.json";

fn get_default_dictionary() -> Dictionary {
    Dictionary { term_infos: FxHashMap::default(), trigrams: FxHashMap::default() }
}

#[derive(Serialize, Deserialize)]
struct DocIdsAndFileHash(
    Vec<u32>,            // doc ids
    u128,                // millis timestamp
    #[serde(skip)] bool, // false by default, detect if files were encountered in the current run (delete if not)
);

#[derive(Serialize, Deserialize)]
pub struct IncrementalIndexInfo {
    pub ver: String,

    pub use_content_hash: bool,

    // Mapping of external doc identifier -> internal doc id(s) / hashes, used for incremental indexing
    mappings: FxHashMap<String, DocIdsAndFileHash>,

    pub last_pl_number: u32,

    pub num_deleted_docs: u32,

    pub pl_names_to_cache: Vec<u32>,

    #[serde(skip)]
    pub invalidation_vector: Vec<u8>,

    #[serde(skip, default = "get_default_dictionary")]
    pub dictionary: Dictionary,
}

impl IncrementalIndexInfo {
    pub fn empty(use_content_hash: bool) -> IncrementalIndexInfo {
        IncrementalIndexInfo {
            ver: MORSELS_VERSION.to_owned(),
            use_content_hash,
            mappings: FxHashMap::default(),
            last_pl_number: 0,
            num_deleted_docs: 0,
            pl_names_to_cache: Vec::new(),
            invalidation_vector: Vec::new(),
            dictionary: get_default_dictionary(),
        }
    }

    pub fn new_from_output_folder(
        output_folder_path: &Path,
        raw_config_normalised: &str,
        is_incremental: &mut bool,
        use_content_hash: bool,
    ) -> IncrementalIndexInfo {
        if !*is_incremental {
            return IncrementalIndexInfo::empty(use_content_hash);
        }

        if let Ok(meta) = std::fs::metadata(output_folder_path.join(INCREMENTAL_INFO_FILE_NAME)) {
            if !meta.is_file() {
                println!("Old incremental index info missing. Running a full reindex.");
                *is_incremental = false;
                return IncrementalIndexInfo::empty(use_content_hash);
            }
        } else {
            println!("Old incremental index info missing. Running a full reindex.");
            *is_incremental = false;
            return IncrementalIndexInfo::empty(use_content_hash);
        }

        if let Ok(mut file) = File::open(output_folder_path.join("old_morsels_config.json")) {
            let mut old_config = "".to_owned();
            file.read_to_string(&mut old_config).expect("Unable to read old config file");
            let old_config_normalised = &String::from_iter(normalized(old_config.chars()));
            if raw_config_normalised != old_config_normalised {
                println!("Configuration file changed. Running a full reindex.");
                *is_incremental = false;
                return IncrementalIndexInfo::empty(use_content_hash);
            }
        } else {
            eprintln!("Old configuration file missing. Running a full reindex.");
            *is_incremental = false;
            return IncrementalIndexInfo::empty(use_content_hash);
        }

        let info_file = File::open(output_folder_path.join(INCREMENTAL_INFO_FILE_NAME)).unwrap();

        let mut info: IncrementalIndexInfo = serde_json::from_reader(BufReader::new(info_file))
            .expect("incremental index info deserialization failed!");

        if &info.ver[..] != MORSELS_VERSION {
            println!("Indexer version changed. Running a full reindex.");
            *is_incremental = false;
            return IncrementalIndexInfo::empty(use_content_hash);
        } else if info.use_content_hash != use_content_hash {
            println!("Content hash option changed. Running a full reindex.");
            *is_incremental = false;
            return IncrementalIndexInfo::empty(use_content_hash);
        }

        // Dictionary
        let mut dictionary_table_vec: Vec<u8> = Vec::new();
        let mut dictionary_string_vec: Vec<u8> = Vec::new();
        File::open(output_folder_path.join(DICTIONARY_TABLE_FILE_NAME))
            .unwrap()
            .read_to_end(&mut dictionary_table_vec)
            .unwrap();
        File::open(output_folder_path.join(DICTIONARY_STRING_FILE_NAME))
            .unwrap()
            .read_to_end(&mut dictionary_string_vec)
            .unwrap();

        info.dictionary = dictionary::setup_dictionary(dictionary_table_vec, dictionary_string_vec, 0, false);

        // Invalidation vector
        File::open(output_folder_path.join(BITMAP_FILE_NAME))
            .unwrap()
            .read_to_end(&mut info.invalidation_vector)
            .unwrap();

        info
    }

    pub fn add_doc_to_file(&mut self, external_id: &str, doc_id: u32) {
        self.mappings
            .get_mut(external_id)
            .expect("Get path for index file should always have an entry when adding doc id")
            .0
            .push(doc_id);
    }

    pub fn set_file(&mut self, external_id: &str, path: &Path) -> bool {
        let new_hash = self.get_file_hash(path);

        if let Some(old_hash) = self.mappings.get_mut(external_id) {
            // Old file

            // Set encountered flag to know which files were deleted later on
            old_hash.2 = true;

            if old_hash.1 != new_hash {
                old_hash.1 = new_hash;

                self.num_deleted_docs += old_hash.0.len() as u32;
                for doc_id in old_hash.0.drain(..) {
                    let byte_num = (doc_id / 8) as usize;
                    self.invalidation_vector[byte_num] |= 1_u8 << (doc_id % 8) as u8;
                }

                return false;
            }

            true
        } else {
            // New file
            self.mappings.insert(external_id.to_owned(), DocIdsAndFileHash(Vec::new(), new_hash, true));

            false
        }
    }

    fn get_file_hash(&self, path: &Path) -> u128 {
        if self.use_content_hash {
            let buf = std::fs::read(path).expect("Failed to read file for calculating content hash!");
            crc32fast::hash(&buf) as u128
        } else {
            // Use last modified timestamp otherwise
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    modified.duration_since(UNIX_EPOCH).unwrap().as_millis()
                } else {
                    /*
                      Use program execution time if metadata is unavailable.
                      This results in the path always being updated.
                    */
                    *CURRENT_MILLIS
                }
            } else {
                *CURRENT_MILLIS
            }
        }
    }

    // Delete file paths that were not encountered at all (assume they were deleted)
    pub fn delete_unencountered_external_ids(&mut self) {
        self.mappings = std::mem::take(&mut self.mappings)
            .into_iter()
            .filter(|(_path, docids_and_filehash)| {
                if !docids_and_filehash.2 {
                    for doc_id in docids_and_filehash.0.iter() {
                        bitmap::set(&mut self.invalidation_vector, *doc_id as usize);
                        self.num_deleted_docs += 1;
                    }
                }

                docids_and_filehash.2
            })
            .collect();
    }

    pub fn write_invalidation_vec(&mut self, output_folder_path: &Path, doc_id_counter: u32) {
        let num_bytes = (doc_id_counter as f64 / 8.0).ceil() as usize;
        
        // Extend with the added documents
        self.invalidation_vector.extend(vec![0; num_bytes - self.invalidation_vector.len()]);

        File::create(output_folder_path.join(BITMAP_FILE_NAME))
            .unwrap()
            .write_all(&*self.invalidation_vector)
            .unwrap();
    }

    pub fn write_info(&mut self, output_folder_path: &Path) {
        let serialized = serde_json::to_string(self).unwrap();

        File::create(output_folder_path.join(INCREMENTAL_INFO_FILE_NAME))
            .unwrap()
            .write_all(serialized.as_bytes())
            .unwrap();
    }
}
