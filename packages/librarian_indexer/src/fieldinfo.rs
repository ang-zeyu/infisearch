use std::fs::File;
use std::io::Write;
use std::path::Path;

use rustc_hash::FxHashMap;

use serde::{Serialize};

#[derive(Serialize, Debug, Clone)]
pub struct FieldInfo {
    pub id: u8,
    pub do_store: bool,
    pub weight: f32,
    pub k: f32,
    pub b: f32,
}

pub struct FieldInfos {
    pub field_infos_map: FxHashMap<String, FieldInfo>,
    pub field_infos_by_id: Vec<FieldInfo>,
    pub num_scored_fields: usize,
}

impl FieldInfos {
    pub fn init(field_infos_map: FxHashMap<String, FieldInfo>) -> FieldInfos {
        let num_scored_fields = field_infos_map.values().filter(|field_info| field_info.weight != 0.0).count();
        
        let mut field_infos_by_id: Vec<FieldInfo> = field_infos_map.values().map(|x| x.clone()).collect();
        field_infos_by_id.sort_by(|fi1, fi2| fi1.id.cmp(&fi2.id));

        FieldInfos {
            field_infos_map,
            field_infos_by_id,
            num_scored_fields
        }
    }

    pub fn dump(&self, output_folder_path: &Path) {
        let serialized = serde_json::to_string(&self.field_infos_map).unwrap();

        File::create(output_folder_path.join("fieldInfo.json"))
            .unwrap()
            .write_all(serialized.as_bytes())
            .unwrap();
    }
}
