use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use morsels_common::{BITMAP_FILE_NAME, bitmap};
use morsels_common::dictionary::{self, Dictionary, DICTIONARY_STRING_FILE_NAME, DICTIONARY_TABLE_FILE_NAME};

use crate::MORSELS_VERSION;

// Not used for search
static DYNAMIC_INDEX_INFO_FILE_NAME: &str = "_dynamic_index_info.json";

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
pub struct DynamicIndexInfo {
    pub ver: String,

    // Mapping of external doc identifier -> internal doc id(s) / hashes, used for dynamic indexing
    mappings: FxHashMap<String, DocIdsAndFileHash>,

    pub last_pl_number: u32,

    pub num_docs: u32,

    pub num_deleted_docs: u32,

    #[serde(skip)]
    pub invalidation_vector: Vec<u8>,

    #[serde(skip, default = "get_default_dictionary")]
    pub dictionary: Dictionary,
}

impl DynamicIndexInfo {
    pub fn empty() -> DynamicIndexInfo {
        DynamicIndexInfo {
            ver: MORSELS_VERSION.to_owned(),
            mappings: FxHashMap::default(),
            last_pl_number: 0,
            num_docs: 0,
            num_deleted_docs: 0,
            invalidation_vector: Vec::new(),
            dictionary: get_default_dictionary(),
        }
    }

    pub fn new_from_output_folder(output_folder_path: &Path, is_dynamic: &mut bool) -> DynamicIndexInfo {
        if let Ok(meta) = std::fs::metadata(output_folder_path.join(DYNAMIC_INDEX_INFO_FILE_NAME)) {
            if !meta.is_file() {
                *is_dynamic = false;
                return DynamicIndexInfo::empty();
            }
        } else {
            *is_dynamic = false;
            return DynamicIndexInfo::empty();
        }

        let info_file = File::open(output_folder_path.join(DYNAMIC_INDEX_INFO_FILE_NAME)).unwrap();

        let mut info: DynamicIndexInfo = serde_json::from_reader(BufReader::new(info_file))
            .expect("dynamic index info deserialization failed!");
        
        if &info.ver[..] != MORSELS_VERSION {
            *is_dynamic = false;
            return DynamicIndexInfo::empty();
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

    pub fn add_doc_to_external_id(&mut self, external_id: &str, doc_id: u32) {
        self.mappings
            .get_mut(external_id)
            .expect("Get path for index file should always have an entry when adding doc id")
            .0
            .push(doc_id);
    }

    pub fn update_doc_if_modified(&mut self, external_id: &str, new_modified: u128) -> bool {
        if let Some(old_modified) = self.mappings.get_mut(external_id) {
            // Old document

            // Set encountered flag to know which files were deleted later on
            old_modified.2 = true;

            if old_modified.1 != new_modified {
                old_modified.1 = new_modified;

                self.num_deleted_docs += old_modified.0.len() as u32;
                for doc_id in old_modified.0.drain(..) {
                    let byte_num = (doc_id / 8) as usize;
                    self.invalidation_vector[byte_num] |= 1_u8 << (doc_id % 8) as u8;
                }

                return true;
            }

            false
        } else {
            // New document
            self.mappings.insert(external_id.to_owned(), DocIdsAndFileHash(Vec::new(), new_modified, true));

            true
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

    pub fn write(&mut self, output_folder_path: &Path, doc_id_counter: u32) {
        let serialized = serde_json::to_string(self).unwrap();

        File::create(output_folder_path.join(DYNAMIC_INDEX_INFO_FILE_NAME))
            .unwrap()
            .write_all(serialized.as_bytes())
            .unwrap();

        let num_bytes = (doc_id_counter as f64 / 8.0).ceil() as usize;
        self.invalidation_vector.extend(vec![0; num_bytes - self.invalidation_vector.len()]);

        File::create(output_folder_path.join(BITMAP_FILE_NAME)).unwrap().write_all(&*self.invalidation_vector).unwrap();
    }
}
