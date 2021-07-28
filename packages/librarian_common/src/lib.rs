use serde::{Serialize,Deserialize};

pub mod tokenize;

fn get_default_language() -> String {
    "latin".to_owned()
}

#[derive(Serialize, Deserialize)]
pub struct LibrarianLanguageConfig {
    #[serde(default = "get_default_language")]
    pub lang: String,

    pub options: Option<serde_json::Value>
}

impl Default for LibrarianLanguageConfig {
    fn default() -> Self {
        LibrarianLanguageConfig {
            lang: get_default_language(),
            options: Option::None,
        }
    }
}
