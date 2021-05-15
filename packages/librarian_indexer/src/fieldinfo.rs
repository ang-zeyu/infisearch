use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use serde::{Serialize};

#[derive(Serialize, Debug)]
pub struct FieldInfo {
    pub id: u8,
    pub do_store: bool,
    pub weight: f32
}

pub type FieldInfos = HashMap<String, FieldInfo>;

pub fn dump_field_infos(field_infos: Arc<FieldInfos>, output_folder_path: &Path) {
    let serialized = serde_json::to_string(&*field_infos).unwrap();

    File::create(output_folder_path.join("fieldInfo.json"))
        .unwrap()
        .write_all(serialized.as_bytes())
        .unwrap();
}
