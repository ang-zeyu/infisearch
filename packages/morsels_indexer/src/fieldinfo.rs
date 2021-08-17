use std::path::PathBuf;
use std::path::Path;

use rustc_hash::FxHashMap;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct FieldsConfig {
    pub field_store_block_size: u32,
    pub fields: Vec<FieldConfig>,
}

impl Default for FieldsConfig {
    fn default() -> Self {
        // The default configuration required for @morsels/search-ui
        FieldsConfig {
            field_store_block_size: 1,
            fields: vec![
                FieldConfig {
                    name: "title".to_owned(),
                    do_store: false,
                    weight: 0.2,
                    k: 1.2,
                    b: 0.25
                },
                FieldConfig {
                    name: "heading".to_owned(),
                    do_store: false,
                    weight: 0.3,
                    k: 1.2,
                    b: 0.3
                },
                FieldConfig {
                    name: "body".to_owned(),
                    do_store: false,
                    weight: 0.5,
                    k: 1.2,
                    b: 0.75
                },
                FieldConfig {
                    name: "headingLink".to_owned(),
                    do_store: false,
                    weight: 0.0,
                    k: 1.2,
                    b: 0.75
                },
                FieldConfig {
                    name: "_relative_fp".to_owned(),
                    do_store: true,
                    weight: 0.0,
                    k: 1.2,
                    b: 0.75
                },
            ]
        }
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

#[derive(Serialize, Debug, Clone)]
pub struct FieldInfo {
    pub id: u8,
    pub do_store: bool,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
}

#[derive(Serialize)]
pub struct FieldInfos {
    pub field_infos_map: FxHashMap<String, FieldInfo>,
    #[serde(skip_serializing)]
    pub field_infos_by_id: Vec<FieldInfo>,
    pub num_scored_fields: usize,
    pub field_store_block_size: u32,
    #[serde(skip_serializing)]
    pub field_output_folder_path: PathBuf,
}

impl FieldInfos {
    pub fn init(
        field_infos_map: FxHashMap<String, FieldInfo>,
        field_store_block_size: u32,
        output_folder_path: &Path
    ) -> FieldInfos {
        let num_scored_fields = field_infos_map.values().filter(|field_info| field_info.weight != 0.0).count();
        
        let mut field_infos_by_id: Vec<FieldInfo> = field_infos_map.values().cloned().collect();
        field_infos_by_id.sort_by(|fi1, fi2| fi1.id.cmp(&fi2.id));

        let field_output_folder_path = output_folder_path.join("field_store");

        std::fs::create_dir_all(&field_output_folder_path).unwrap();

        FieldInfos {
            field_infos_map,
            field_infos_by_id,
            num_scored_fields,
            field_store_block_size,
            field_output_folder_path,
        }
    }
}
