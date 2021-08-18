use serde::{Serialize,Deserialize};

pub mod bitmap;
pub mod dictionary;
pub mod tokenize;
pub mod utils;

pub static DOC_INFO_FILE_NAME: &str = "_doc_info";
pub static BITMAP_FILE_NAME: &str = "_invalidation_vector";

fn get_default_language() -> String {
    "latin".to_owned()
}

#[derive(Serialize, Deserialize)]
pub struct MorselsLanguageConfig {
    #[serde(default = "get_default_language")]
    pub lang: String,

    pub options: Option<serde_json::Value>
}

impl Default for MorselsLanguageConfig {
    fn default() -> Self {
        MorselsLanguageConfig {
            lang: get_default_language(),
            options: Option::None,
        }
    }
}
