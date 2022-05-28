use std::cmp::Ordering;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use serde::{Deserialize, Serialize};

fn get_default_num_field_stores_per_dir() -> u32 {
    1000
}

fn get_default_cache_all_field_stores() -> bool {
    true
}

// Raw Json field configuration
#[derive(Serialize, Deserialize)]
pub struct FieldsConfig {
    pub field_store_block_size: u32,
    #[serde(default = "get_default_num_field_stores_per_dir")]
    pub num_stores_per_dir: u32,
    #[serde(default="get_default_cache_all_field_stores")]
    pub cache_all_field_stores: bool,
    pub fields: Vec<FieldConfig>,
}

impl Default for FieldsConfig {
    fn default() -> Self {
        // The default configuration required for @morsels/search-ui
        FieldsConfig {
            field_store_block_size: 10000,
            num_stores_per_dir: get_default_num_field_stores_per_dir(),
            cache_all_field_stores: get_default_cache_all_field_stores(),
            fields: vec![
                FieldConfig { name: "title".to_owned(), do_store: false, weight: 0.6, k: 1.2, b: 0.25 },
                FieldConfig { name: "heading".to_owned(), do_store: false, weight: 0.6, k: 1.2, b: 0.3 },
                FieldConfig { name: "body".to_owned(), do_store: false, weight: 1.0, k: 1.2, b: 0.75 },
                FieldConfig { name: "headingLink".to_owned(), do_store: false, weight: 0.0, k: 1.2, b: 0.75 },
                FieldConfig { name: "_relative_fp".to_owned(), do_store: true, weight: 0.0, k: 1.2, b: 0.75 },
            ],
        }
    }
}

impl FieldsConfig {
    pub fn get_field_infos(&self, output_folder_path: &Path) -> Arc<FieldInfos> {
        let mut field_infos_by_name: FxHashMap<String, FieldInfo> = FxHashMap::default();
        for field_config in self.fields.iter() {
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

        // Larger-weight fields are assigned lower ids
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

        for (field_id, (_, field_info)) in field_entries.iter_mut().enumerate() {
            field_info.id = field_id as u8;
        }

        Arc::new(FieldInfos::init(
            field_infos_by_name,
            self.field_store_block_size,
            self.num_stores_per_dir,
            output_folder_path,
        ))
    }
}

#[derive(Serialize, Deserialize)]
pub struct FieldConfig {
    pub name: String,
    pub do_store: bool,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub id: u8,
    pub do_store: bool,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
}

// Initialised json field configuration
pub struct FieldInfos {
    pub field_infos_map: FxHashMap<String, FieldInfo>,

    pub field_infos_by_id: Vec<FieldInfo>,

    pub num_scored_fields: usize,

    pub field_store_block_size: u32,

    pub num_stores_per_dir: u32,

    pub field_output_folder_path: PathBuf,
}

#[derive(Serialize)]
pub struct FieldInfoOutput {
    pub id: u8,
    pub name: String,
    pub do_store: bool,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
}

impl FieldInfos {
    pub fn init(
        field_infos_map: FxHashMap<String, FieldInfo>,
        field_store_block_size: u32,
        num_stores_per_dir: u32,
        output_folder_path: &Path,
    ) -> FieldInfos {
        let num_scored_fields = field_infos_map
            .values()
            .filter(|field_info| field_info.weight != 0.0)
            .count();

        let mut field_infos_by_id: Vec<FieldInfo> = field_infos_map.values().cloned().collect();
        field_infos_by_id.sort_by(|fi1, fi2| fi1.id.cmp(&fi2.id));

        let field_output_folder_path = output_folder_path.join("field_store");

        std::fs::create_dir_all(&field_output_folder_path).unwrap();

        FieldInfos {
            field_infos_map,
            field_infos_by_id,
            num_scored_fields,
            field_store_block_size,
            num_stores_per_dir,
            field_output_folder_path,
        }
    }

    pub fn to_output(&self) -> Vec<FieldInfoOutput> {
        let mut field_infos: Vec<FieldInfoOutput> = Vec::with_capacity(self.field_infos_map.len());

        for (field_name, field_info) in self.field_infos_map.iter() {
            field_infos.push(FieldInfoOutput {
                id: field_info.id,
                name: field_name.to_owned(),
                do_store: field_info.do_store,
                weight: field_info.weight,
                k: field_info.k, b: field_info.b,
            })
        }

        field_infos.sort_by(|field_info_1, field_info_2| field_info_1.id.cmp(&field_info_2.id));

        field_infos
    }
}
