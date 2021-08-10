use serde::{Serialize,Deserialize};

pub mod tokenize;

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
