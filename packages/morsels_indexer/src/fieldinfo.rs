use std::cmp::Ordering;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use serde::{Deserialize, Serialize};

use crate::indexer::output_config::MorselsOutputConfig;

pub static RELATIVE_FP_FIELD: &str = "_relative_fp";
pub static ADD_FILES_FIELD: &str = "_add_files";

pub type EnumKind = String;

fn get_default_num_docs_per_store() -> u32 {
    100000000
}

fn get_default_num_field_stores_per_dir() -> u32 {
    1000
}

fn get_default_cache_all_field_stores() -> bool {
    true
}

fn get_default_fields() -> Vec<FieldConfig> {
    vec![
        FieldConfig {
            name: "title".to_owned(),
            storage: get_default_storage(),
            weight: 2.0, k: 1.2, b: 0.15
        },
        FieldConfig {
            name: "h1".to_owned(),
            storage: get_default_storage(),
            weight: 2.0, k: 1.2, b: 0.15
        },
        FieldConfig {
            name: "heading".to_owned(),
            storage: get_default_storage(),
            weight: 1.5, k: 1.2, b: 0.25
        },
        FieldConfig {
            name: "body".to_owned(),
            storage: get_default_storage(),
            weight: 1.0, k: 1.2, b: 0.75
        },
        FieldConfig {
            name: "headingLink".to_owned(),
            storage: get_default_storage(),
            weight: 0.0, k: 1.2, b: 0.75
        },
        FieldConfig {
            name: RELATIVE_FP_FIELD.to_owned(),
            storage: get_default_storage(),
            weight: 0.0, k: 1.2, b: 0.75
        },
        FieldConfig {
            name: "link".to_owned(),
            storage: get_default_storage(),
            weight: 0.0, k: 1.2, b: 0.75
        },
    ]
}

// Raw Json field configuration
#[derive(Serialize, Deserialize)]
pub struct FieldsConfig {
    #[serde(default = "get_default_num_docs_per_store")]
    pub num_docs_per_store: u32,
    #[serde(default = "get_default_num_field_stores_per_dir")]
    pub num_stores_per_dir: u32,
    #[serde(default="get_default_cache_all_field_stores")]
    pub cache_all_field_stores: bool,
    #[serde(default="get_default_fields")]
    pub fields: Vec<FieldConfig>,
}

impl Default for FieldsConfig {
    fn default() -> Self {
        // The default configuration required for @morsels/search-ui
        FieldsConfig {
            num_docs_per_store: get_default_num_docs_per_store(),
            num_stores_per_dir: get_default_num_field_stores_per_dir(),
            cache_all_field_stores: get_default_cache_all_field_stores(),
            fields: get_default_fields(),
        }
    }
}

impl FieldsConfig {
    pub fn get_field_infos(&self, output_folder_path: &Path, is_incremental: bool) -> Arc<FieldInfos> {
        let mut field_infos_by_name: FxHashMap<String, FieldInfo> = FxHashMap::default();

        let old_config = if is_incremental {
            let old_output_conf_str = std::fs::read_to_string(output_folder_path.join("morsels_config.json")).unwrap();
            let old_output_conf: MorselsOutputConfig = serde_json::from_str(&old_output_conf_str).unwrap();
            old_output_conf.field_infos
        } else {
            Vec::new()
        };

        let mut num_enum_fields = 0;
        for field_config in self.fields.iter() {
            field_infos_by_name.insert(
                field_config.name.to_owned(),
                FieldInfo {
                    id: 0,
                    enum_info: if field_config.storage.iter().any(|s| s == "enum") {
                        let enum_id = num_enum_fields;
                        num_enum_fields += 1;

                        let enum_values = old_config.iter()
                            .find_map(|field_info_output| if let Some(EnumInfo {
                                enum_id: curr_enum_id, enum_values
                            }) = &field_info_output.enum_info {
                                if *curr_enum_id == enum_id {
                                    Some(enum_values.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            })
                            .unwrap_or_else(Vec::new);

                        Some(EnumInfo {
                            enum_id,
                            enum_values,
                        })
                    } else {
                        None
                    },
                    store_text: field_config.storage.iter().any(|s| s == "text"),
                    weight: field_config.weight,
                    k: field_config.k,
                    b: field_config.b,
                },
            );
        }

        // Larger-weight fields are assigned lower ids
        // Stable sort to preserve incremental indexing field order
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

        for (field_id, (_, field_info)) in field_entries.into_iter().enumerate() {
            field_info.id = field_id as u8;
        }

        Arc::new(FieldInfos::init(
            field_infos_by_name,
            self.num_docs_per_store,
            num_enum_fields,
            self.num_stores_per_dir,
            output_folder_path,
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnumInfo {
    pub enum_id: usize,
    pub enum_values: Vec<String>,
}

fn get_default_k() -> f32 {
    1.2
}

fn get_default_b() -> f32 {
    0.75
}

fn get_default_storage() -> Vec<String> {
    vec!["text".to_owned()]
}

#[derive(Serialize, Deserialize)]
pub struct FieldConfig {
    pub name: String,
    #[serde(default = "get_default_storage")]
    pub storage: Vec<String>,
    pub weight: f32,
    #[serde(default = "get_default_k")]
    pub k: f32,
    #[serde(default = "get_default_b")]
    pub b: f32,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub id: u8,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
    pub enum_info: Option<EnumInfo>,
    pub store_text: bool,
}

// Initialised json field configuration
pub struct FieldInfos {
    pub field_infos_map: FxHashMap<String, FieldInfo>,

    pub field_infos_by_id: Vec<FieldInfo>,

    pub num_scored_fields: usize,

    pub num_enum_fields: usize,

    pub num_docs_per_store: u32,

    pub num_stores_per_dir: u32,

    pub field_output_folder_path: PathBuf,
}

/// Separate struct from FieldInfo to add in the name
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldInfoOutput {
    pub id: u8,
    pub name: String,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
    pub store_text: bool,
    pub enum_info: Option<EnumInfo>,
}

impl FieldInfos {
    pub fn init(
        field_infos_map: FxHashMap<String, FieldInfo>,
        num_docs_per_store: u32,
        num_enum_fields: usize,
        num_stores_per_dir: u32,
        output_folder_path: &Path,
    ) -> FieldInfos {
        let num_scored_fields = field_infos_map
            .values()
            .filter(|&field_info| field_info.weight != 0.0)
            .count();

        let mut field_infos_by_id: Vec<FieldInfo> = field_infos_map.values().cloned().collect();
        field_infos_by_id.sort_by_key(|fi| fi.id);

        let field_output_folder_path = output_folder_path.join("field_store");

        std::fs::create_dir_all(&field_output_folder_path)
            .expect("Failed to create field store output folder in output directory");

        FieldInfos {
            field_infos_map,
            field_infos_by_id,
            num_scored_fields,
            num_enum_fields,
            num_docs_per_store,
            num_stores_per_dir,
            field_output_folder_path,
        }
    }

    pub fn to_output(&self) -> Vec<FieldInfoOutput> {
        let mut field_infos: Vec<FieldInfoOutput> = Vec::with_capacity(self.field_infos_map.len());

        for (field_name, field_info) in self.field_infos_map.iter() {
            field_infos.push(FieldInfoOutput {
                id: field_info.id,
                enum_info: field_info.enum_info.clone(),
                name: field_name.to_owned(),
                store_text: field_info.store_text,
                weight: field_info.weight,
                k: field_info.k, b: field_info.b,
            })
        }

        field_infos.sort_by_key(|fi| fi.id);

        field_infos
    }
}
