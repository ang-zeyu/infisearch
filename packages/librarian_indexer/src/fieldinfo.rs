use std::collections::HashMap;

pub struct FieldInfo {
    pub id: u8,
    pub storage: String,
    pub storage_params: HashMap<String, String>,
    pub weight: f32
}