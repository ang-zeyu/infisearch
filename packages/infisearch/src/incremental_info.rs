use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{info, warn};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use infisearch_common::dictionary::Dictionary;
use infisearch_common::{bitmap, METADATA_FILE};
use infisearch_common::metadata::MetadataReader;

use crate::indexer::output_config::InfiOutputConfig;
use crate::utils::fs_utils;
use crate::{INFISEARCH_VER, i_debug, OLD_SOURCE_CONFIG, OUTPUT_CONFIG_FILE};

lazy_static! {
    static ref CURRENT_MILLIS: u128 = SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("Failed to obtain current system time. Consider using the --incremental-content-hash option.")
        .as_millis();
}

// Not used for search
static INCREMENTAL_INFO_FILE_NAME: &str = "_incremental_info.json";

fn get_default_dictionary() -> Dictionary {
    Dictionary { term_infos: BTreeMap::default() }
}

// TODO write a custom serialize-deserialize with a named struct for readability
#[derive(Serialize, Deserialize)]
struct DocIdsAndFileHash(
    Vec<u32>,            // doc ids
    u32,                 // hash
    #[serde(skip)] bool, // false by default, detect if files were encountered in the current run (delete if not)
    Vec<String>,         // secondary files that were _add_files linked to
);

// Special hash to indicate a file does not exist
// If it turns out a file organically has this hash, the worst case is that we re-index it.
static ABSENT_HASH: u32 = 126487710;

#[derive(Serialize, Deserialize)]
pub struct IncrementalIndexInfo {
    pub use_content_hash: bool,

    // Mapping of external doc identifier -> internal doc id(s) / hashes / secondary files
    mappings: FxHashMap<String, DocIdsAndFileHash>,

    // Mapping of internal doc id(s) -> external doc identifier
    #[serde(skip)]
    inv_mappings: FxHashMap<u32, String>,

    // Mapping of internal doc id(s) -> secondary files' identifiers
    #[serde(skip)]
    inv_mappings_secondary: Vec<FxHashMap<u32, Vec<String>>>,

    pub last_pl_number: u32,

    pub num_deleted_docs: u32,

    pub pl_names_to_cache: Vec<u32>,

    #[serde(skip)]
    pub invalidation_vector: Vec<u8>,

    #[serde(skip, default = "get_default_dictionary")]
    pub dictionary: Dictionary,
}

impl IncrementalIndexInfo {
    pub fn empty(use_content_hash: bool) -> (
        Option<InfiOutputConfig>,
        Option<MetadataReader>,
        IncrementalIndexInfo,
    ) {
        (
            None,
            None,
            IncrementalIndexInfo {
                use_content_hash,
                mappings: FxHashMap::default(),
                inv_mappings: FxHashMap::default(),
                inv_mappings_secondary: Vec::new(),
                last_pl_number: 0,
                num_deleted_docs: 0,
                pl_names_to_cache: Vec::new(),
                invalidation_vector: Vec::new(),
                dictionary: get_default_dictionary(),
            },
        )
    }

    pub fn new_from_output_folder(
        output_folder_path_inner: &Path,
        output_folder_path: &Path,
        json_config: &Value,
        is_incremental: bool,
        use_content_hash: bool,
    ) -> (Option<InfiOutputConfig>, Option<MetadataReader>, IncrementalIndexInfo) {
        // --------------------------------------------------------
        // Full index
        if !is_incremental {
            return IncrementalIndexInfo::empty(use_content_hash);
        }
        // --------------------------------------------------------

        // --------------------------------------------------------
        // Check for old incremental index info file as a first start
        // to whether there was any previous index runs.
        if let Ok(meta) = std::fs::metadata(output_folder_path.join(INCREMENTAL_INFO_FILE_NAME)) {
            if !meta.is_file() {
                info!("Old incremental index info missing. Running a full reindex.");
                return IncrementalIndexInfo::empty(use_content_hash);
            }
        } else {
            info!("Old incremental index info missing. Running a full reindex.");
            return IncrementalIndexInfo::empty(use_content_hash);
        }
        // --------------------------------------------------------

        // --------------------------------------------------------
        // Check for the old output config file, which should be here at this point.
        // The InfiSearch version used might however be different.
        let old_output_config = output_folder_path.join(OUTPUT_CONFIG_FILE);
        let old_output_config = if old_output_config.exists() {
            let old_output_conf_str = std::fs::read_to_string(&old_output_config).unwrap();
            let deserialized: Result<InfiOutputConfig, _> = serde_json::from_str(&old_output_conf_str);

            if let Ok(old_output_conf) = deserialized {
                if old_output_conf.ver != INFISEARCH_VER {
                    info!("InfiSearch version changed. Running a full reindex.");
                    return IncrementalIndexInfo::empty(use_content_hash);
                }

                old_output_conf
            } else {
                info!("Old output config invalid. Running a full reindex.");
                return IncrementalIndexInfo::empty(use_content_hash);
            }
        } else {
            warn!("Old output config missing. Running a full reindex.");
            return IncrementalIndexInfo::empty(use_content_hash);
        };
        // --------------------------------------------------------

        // --------------------------------------------------------
        // Next, verify the user config hasn't changed.
        if let Ok(mut file) = File::open(output_folder_path.join(OLD_SOURCE_CONFIG)) {
            let mut old_config = "".to_owned();
            file.read_to_string(&mut old_config).expect("Unable to read old config file");
            let old_json_config: Value = serde_json::from_str(&old_config)
                .expect(&(OLD_SOURCE_CONFIG.to_owned() + " does not match schema!"));
            if *json_config != old_json_config {
                info!("Configuration file changed. Running a full reindex.");
                return IncrementalIndexInfo::empty(use_content_hash);
            }
        } else {
            warn!("Old configuration file missing. Running a full reindex.");
            return IncrementalIndexInfo::empty(use_content_hash);
        }
        // --------------------------------------------------------

        // --------------------------------------------------------
        // Check the type of content hash used is the same
        let info_file = File::open(output_folder_path.join(INCREMENTAL_INFO_FILE_NAME))
            .expect("Failed to obtain incremental index info file handle.");

        let mut info: IncrementalIndexInfo = serde_json::from_reader(BufReader::new(info_file))
            .expect("incremental index info deserialization failed!");

        if info.use_content_hash != use_content_hash {
            info!("Content hash option changed. Running a full reindex.");
            return IncrementalIndexInfo::empty(use_content_hash);
        }
        // --------------------------------------------------------

        // --------------------------------------------------------
        // Move the old postings lists, field stores etc. over
        let old_output_folder_inner = output_folder_path.join(&old_output_config.index_ver); 
        if let Err(e) = std::fs::rename(&old_output_folder_inner, &output_folder_path_inner) {
            // Sometimes fragile due to antivirus
            // Retry with copy-paste then remove
            info!("Failed to rename old output folder, trying copy-paste & removal.");
            if let Ok(_) = dircpy::copy_dir(&old_output_folder_inner, &output_folder_path_inner) {
                fs_utils::clean_dir(&old_output_folder_inner);
            } else {
                warn!(
                    "Failed to rename old output folder, performing a full index.\n  Old: {}\n  New: {}\n  Cause: {}",
                    old_output_folder_inner.to_string_lossy(),
                    output_folder_path_inner.to_string_lossy(),
                    e,
                );
                return IncrementalIndexInfo::empty(use_content_hash);
            }
        }
        // --------------------------------------------------------

        // --------------------------------------------------------
        // Initialise metadata, read in the invalidation vector

        let metadata_rdr = if let Ok(mut file) = File::open(output_folder_path_inner.join(METADATA_FILE)) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            MetadataReader::new(buf)
        } else {
            warn!("metadata file missing. Running a full reindex.");
            return IncrementalIndexInfo::empty(use_content_hash);
        };

        metadata_rdr.get_invalidation_vec(&mut info.invalidation_vector);

        // --------------------------------------------------------

        (Some(old_output_config), Some(metadata_rdr), info)
    }

    pub fn setup_dictionary(&mut self, metadata_rdr: &MetadataReader) {
        self.dictionary = metadata_rdr.setup_dictionary();
    }

    pub fn add_doc_to_file(&mut self, external_id: &str, doc_id: u32) {
        self.mappings
            .get_mut(external_id)
            .expect("Get path for index file should always have an entry when adding doc id")
            .0
            .push(doc_id);

        self.inv_mappings.insert(doc_id, external_id.to_owned());
    }

    /// Returns whether file was not modified or not for incremental indexing.
    /// A new file is counted as "modified"
    pub fn set_file(&mut self, external_id: &str, path: &Path, input_folder_path: &Path) -> bool {
        if let Some(old_hash) = self.mappings.get_mut(external_id) {
            // Old file
            let new_hash = Self::get_file_hash(
                self.use_content_hash,
                path,
                input_folder_path,
                &old_hash.3,
            );

            // Set encountered flag to know which files were deleted later on
            old_hash.2 = true;

            if old_hash.1 != new_hash {
                i_debug!("{} was updated", external_id);

                self.num_deleted_docs += old_hash.0.len() as u32;
                for doc_id in old_hash.0.drain(..) {
                    bitmap::set(&mut self.invalidation_vector, doc_id as usize);
                }
                old_hash.3.clear();

                return false;
            }

            true
        } else {
            // New file
            self.mappings.insert(external_id.to_owned(), DocIdsAndFileHash(Vec::new(), 0, true, Vec::new()));

            false
        }
    }

    pub fn extend_secondary_inv_mappings(&mut self, mappings: Vec<FxHashMap<u32, Vec<String>>>) {
        self.inv_mappings_secondary.extend(mappings);
    }

    fn get_timestamp(path: &Path) -> u128 {
        if !path.is_file() {
            return ABSENT_HASH as u128;
        }

        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                modified.duration_since(UNIX_EPOCH)
                    .expect("Failed to calculate timestamp. Consider using the --incremental-content-hash option")
                    .as_millis()
            } else {
                i_debug!("Obtaining modified timestamp failed for {}", path.to_string_lossy());

                /*
                  Use program execution time if metadata is unavailable.
                  This results in the path always being updated.
                */
                *CURRENT_MILLIS
            }
        } else {
            i_debug!("Obtaining metadata failed for {}", path.to_string_lossy());

            *CURRENT_MILLIS
        }
    }

    fn get_file_hash(
        use_content_hash: bool,
        path: &Path,
        input_folder_path: &Path,
        secondary_paths: &Vec<String>,
    ) -> u32 {
        if use_content_hash {
            static ERR: &str = "Failed to read file for calculating content hash!";

            let mut buf = if path.is_file() {
                std::fs::read(path).expect(ERR)
            } else {
                ABSENT_HASH.to_le_bytes().to_vec()
            };

            for secondary_path in secondary_paths {
                let secondary_path = input_folder_path.join(secondary_path);
                if secondary_path.is_file() {
                    File::open(secondary_path)
                        .expect(ERR)
                        .read_to_end(&mut buf)
                        .expect(ERR);
                } else {
                    buf.extend_from_slice(&ABSENT_HASH.to_le_bytes());
                }
            }

            crc32fast::hash(&buf)
        } else {
            // Use last modified timestamp otherwise
            let mut timestamps = Vec::with_capacity(1 + secondary_paths.len());
            timestamps.push(Self::get_timestamp(path));

            for secondary_path in secondary_paths {
                timestamps.push(Self::get_timestamp(&input_folder_path.join(secondary_path)));
            }

            crc32fast::hash(unsafe {
                std::slice::from_raw_parts(timestamps.as_ptr() as *const u8, timestamps.len() * 16)
            })
        }
    }

    // Delete file paths that were not encountered at all (assume they were deleted)
    pub fn delete_unencountered_external_ids(&mut self) {
        self.mappings = std::mem::take(&mut self.mappings)
            .into_iter()
            .filter(|(_path, docids_and_filehash)| {
                if !docids_and_filehash.2 {
                    i_debug!("{} was deleted", _path);

                    for &doc_id in docids_and_filehash.0.iter() {
                        bitmap::set(&mut self.invalidation_vector, doc_id as usize);
                        self.num_deleted_docs += 1;
                    }
                }

                docids_and_filehash.2
            })
            .collect();
    }

    pub fn write_invalidation_vec(&mut self, doc_id_counter: u32) -> Vec<u8> {
        let num_bytes = (doc_id_counter as f64 / 8.0).ceil() as usize;
        
        // Extend with the added documents
        self.invalidation_vector.extend(vec![0; num_bytes - self.invalidation_vector.len()]);

        self.invalidation_vector.clone()
    }

    fn update_file_hashes(&mut self, input_folder_path: &Path) {
        for map in std::mem::take(&mut self.inv_mappings_secondary) {
            for (doc_id, secondary_ids) in map {
                let main_id = self.inv_mappings.get(&doc_id)
                    .expect("Inverse mapping should contain doc_id");
                let doc_id_and_filehash = self.mappings.get_mut(main_id)
                    .expect("Mappings should contain main_id");

                doc_id_and_filehash.3.extend(secondary_ids);
            }
        }

        for (main_id, doc_id_and_filehash) in self.mappings.iter_mut() {
            doc_id_and_filehash.1 = Self::get_file_hash(
                self.use_content_hash,
                &input_folder_path.join(main_id),
                input_folder_path,
                &doc_id_and_filehash.3,
            );
        }
    }

    pub fn write_info(&mut self, input_folder_path: &Path, output_folder_path: &Path) {
        self.update_file_hashes(input_folder_path);

        let serialized = serde_json::to_string(self).unwrap();

        File::create(output_folder_path.join(INCREMENTAL_INFO_FILE_NAME))
            .unwrap()
            .write_all(serialized.as_bytes())
            .unwrap();
    }
}
