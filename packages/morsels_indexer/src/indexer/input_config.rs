
use morsels_common::MorselsLanguageConfig;

use crate::fieldinfo::FieldsConfig;
use crate::loader::Loader;
use crate::loader::csv::CsvLoader;
use crate::loader::html::HtmlLoader;
use crate::loader::json::JsonLoader;
use crate::loader::txt::TxtLoader;

use glob::Pattern;
use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};

fn get_default_num_threads() -> usize {
    std::cmp::max(num_cpus::get_physical() - 1, 1)
}

fn get_default_pl_limit() -> u32 {
    5242880
}

fn get_default_num_docs_per_block() -> u32 {
    1000
}

fn get_default_pl_cache_threshold() -> u32 {
    0
}

fn get_default_exclude_patterns() -> Vec<String> {
    vec!["morsels_config.json".to_owned()]
}

fn get_default_loader_configs() -> FxHashMap<String, serde_json::Value> {
    let mut configs = FxHashMap::default();

    configs.insert("HtmlLoader".to_owned(), serde_json::json!({}));

    configs
}

fn get_default_num_pls_per_dir() -> u32 {
    1000
}

fn get_default_with_positions() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct MorselsIndexingConfig {
    #[serde(default = "get_default_num_threads", skip_serializing)]
    pub num_threads: usize,

    #[serde(default = "get_default_num_docs_per_block")]
    pub num_docs_per_block: u32,

    #[serde(default = "get_default_pl_limit")]
    pub pl_limit: u32,

    #[serde(default = "get_default_pl_cache_threshold")]
    pub pl_cache_threshold: u32,

    #[serde(default = "get_default_exclude_patterns")]
    pub exclude: Vec<String>,

    #[serde(skip, default = "Vec::new")]
    pub exclude_patterns: Vec<Pattern>,

    #[serde(default = "get_default_loader_configs")]
    pub loader_configs: FxHashMap<String, serde_json::Value>,

    #[serde(default = "get_default_num_pls_per_dir")]
    pub num_pls_per_dir: u32,

    #[serde(default = "get_default_with_positions")]
    pub with_positions: bool,
}

impl Default for MorselsIndexingConfig {
    fn default() -> Self {
        let mut indexing_config = MorselsIndexingConfig {
            num_threads: get_default_num_threads(),
            num_docs_per_block: get_default_num_docs_per_block(),
            pl_limit: get_default_pl_limit(),
            pl_cache_threshold: get_default_pl_cache_threshold(),
            exclude: get_default_exclude_patterns(),
            exclude_patterns: Vec::new(),
            loader_configs: get_default_loader_configs(),
            num_pls_per_dir: get_default_num_pls_per_dir(),
            with_positions: get_default_with_positions(),
        };

        indexing_config.init_excludes();
        indexing_config
    }
}

impl MorselsIndexingConfig {
    pub fn get_loaders_from_config(&self) -> Vec<Box<dyn Loader>> {
        let mut loaders: Vec<Box<dyn Loader>> = Vec::new();

        for (key, value) in self.loader_configs.clone() {
            match &key[..] {
                "HtmlLoader" => loaders.push(HtmlLoader::get_new_html_loader(value)),
                "CsvLoader" => loaders.push(CsvLoader::get_new_csv_loader(value)),
                "JsonLoader" => loaders.push(JsonLoader::get_new_json_loader(value)),
                "TxtLoader" => loaders.push(TxtLoader::get_new_txt_loader(value)),
                _ => panic!("Unknown loader type encountered in config"),
            }
        }

        loaders
    }

    pub fn init_excludes(&mut self) {
        self.exclude_patterns = self.exclude
            .iter()
            .map(|pat_str| Pattern::new(pat_str).expect("Invalid exclude glob pattern!"))
            .collect();
    }
}

#[derive(Serialize, Deserialize)]
pub struct MorselsConfig {
    #[serde(default)]
    pub fields_config: FieldsConfig,
    #[serde(default)]
    pub lang_config: MorselsLanguageConfig,
    #[serde(default)]
    pub indexing_config: MorselsIndexingConfig,
    #[serde(skip)]
    pub raw_config: String,
}

impl MorselsConfig {
    pub fn new(raw_config: String) -> Self {
        let mut config: MorselsConfig = serde_json::from_str(&raw_config)
            .expect("morsels_config.json does not match schema!");
        config.raw_config = raw_config;
        config.indexing_config.init_excludes();
        config
    }
}

impl Default for MorselsConfig {
    fn default() -> Self {
        MorselsConfig {
            indexing_config: MorselsIndexingConfig::default(),
            lang_config: MorselsLanguageConfig::default(),
            fields_config: FieldsConfig::default(),
            raw_config: "".to_owned(),
        }
    }
}
