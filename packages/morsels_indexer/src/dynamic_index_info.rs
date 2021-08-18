use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use serde::{Serialize, Deserialize};
use rustc_hash::FxHashMap;

// Not used for search
pub static DYNAMIC_INDEX_INFO_FILE_NAME: &str = "_dynamic_index_info.json";

// Used during search and indexing
static BITMAP_FILE_NAME: &str = "_invalidation_vector";

#[derive(Serialize, Deserialize)]
struct DocIdsAndFileHash(
    Vec<u32>, // doc ids
    u128,     // millis timestamp
    #[serde(skip)]
    bool,     // false by default, detect if files were encountered in the current run (delete if not)
);

#[derive(Serialize, Deserialize)]
pub struct DynamicIndexInfo {
    // Mapping of file path -> doc id(s) / file hases, used for dynamic indexing
    mappings: FxHashMap<String, DocIdsAndFileHash>,

    pub last_pl_number: u32,

    pub num_docs: u32,

    pub num_deleted_docs: u32,

    #[serde(skip)]
    pub invalidation_vector: Vec<u8>,
}

impl DynamicIndexInfo {
    pub fn empty() -> DynamicIndexInfo {
        DynamicIndexInfo {
            mappings: FxHashMap::default(),
            last_pl_number: 0,
            num_docs: 0,
            num_deleted_docs: 0,
            invalidation_vector: Vec::new(),
        }
    }

    pub fn new_from_output_folder(output_folder_path: &Path) -> DynamicIndexInfo {
        let info_file = File::open(output_folder_path.join(DYNAMIC_INDEX_INFO_FILE_NAME)).unwrap();

        let mut info: DynamicIndexInfo = serde_json::from_reader(BufReader::new(info_file))
            .expect("dynamic index info deserialization failed!");

        File::open(output_folder_path.join(BITMAP_FILE_NAME)).unwrap()
            .read_to_end(&mut info.invalidation_vector).unwrap();

        info
    }

    pub fn add_doc_to_path(&mut self, path: &Path, doc_id: u32) {
        let path = path.to_str().unwrap();
        self.mappings.get_mut(path)
            .expect("Get path for index file should always have an entry when adding doc id")
            .0.push(doc_id);
    }

    pub fn update_path_if_modified(&mut self, path: &Path, new_modified: u128) -> bool {
        let path = path.to_str().unwrap();
        if let Some(old_modified) = self.mappings.get_mut(path) {
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
            self.mappings.insert(path.to_owned(), DocIdsAndFileHash(Vec::new(), new_modified, true));

            true
        }
    }

    // Delete file paths that were not encountered at all (assume they were deleted)
    pub fn delete_unencountered_paths(&mut self) {
        self.mappings = std::mem::take(&mut self.mappings).into_iter()
            .filter(|(_path, docids_and_filehash)| {
                if !docids_and_filehash.2 {
                    for doc_id in docids_and_filehash.0.iter() {
                        let byte_num = ((*doc_id) / 8) as usize;
                        self.invalidation_vector[byte_num] |= 1_u8 << ((*doc_id) % 8) as u8;
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

        File::create(output_folder_path.join(BITMAP_FILE_NAME))
            .unwrap()
            .write_all(&*self.invalidation_vector)
            .unwrap();
    }
}
