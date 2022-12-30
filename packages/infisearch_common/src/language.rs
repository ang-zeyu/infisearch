#[cfg(feature = "indexer")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "indexer")]
fn get_default_language() -> String {
    "ascii".to_owned()
}

#[cfg_attr(feature = "indexer", derive(Serialize, Deserialize, Clone))]
pub struct InfiLanguageConfigOpts {
    pub stop_words: Option<Vec<String>>,
    pub ignore_stop_words: Option<bool>,
    pub stemmer: Option<String>,
    pub max_term_len: Option<usize>,
}

#[cfg(feature = "indexer")]
impl Default for InfiLanguageConfigOpts {
    fn default() -> Self {
        InfiLanguageConfigOpts {
            stop_words: None,
            ignore_stop_words: None,
            stemmer: None,
            max_term_len: None,
        }
    }
}

#[cfg_attr(feature = "indexer", derive(Serialize, Deserialize, Clone))]
pub struct InfiLanguageConfig {
    #[cfg_attr(feature = "indexer", serde(default = "get_default_language"))]
    pub lang: String,

    #[cfg_attr(feature = "indexer", serde(default))]
    pub options: InfiLanguageConfigOpts,
}

#[cfg(feature = "indexer")]
impl Default for InfiLanguageConfig {
    fn default() -> Self {
        InfiLanguageConfig {
            lang: get_default_language(),
            options: InfiLanguageConfigOpts::default(),
        }
    }
}
