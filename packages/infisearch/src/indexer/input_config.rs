mod preset;
mod preset_small;
mod preset_medium;
mod preset_large;

use std::path::Path;

use infisearch_common::language::InfiLanguageConfig;

use crate::{field_info::FieldsConfig, SOURCE_CONFIG_FILE};
use crate::loader::LoaderBoxed;
use crate::loader::csv::CsvLoader;
use crate::loader::html::HtmlLoader;
use crate::loader::json::JsonLoader;
use crate::loader::txt::TxtLoader;
use crate::loader::pdf::PdfLoader;

use glob::Pattern;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde_json::Value;

fn get_default_preset() -> String {
    "small".to_owned()
}

fn get_default_num_threads() -> usize {
    std::cmp::max(std::cmp::min(num_cpus::get_physical(), num_cpus::get()) - 1, 1)
}

fn get_default_pl_limit() -> u32 {
    std::u32::MAX
}

fn get_default_num_docs_per_block() -> u32 {
    1000
}

fn get_default_pl_cache_threshold() -> u32 {
    0
}

fn get_default_exclude_patterns() -> Vec<String> {
    vec![SOURCE_CONFIG_FILE.to_owned()]
}

fn get_default_loaders() -> FxHashMap<String, serde_json::Value> {
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

#[derive(Deserialize)]
pub struct InfiIndexingConfig {
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

    #[serde(default = "Vec::new")]
    pub include: Vec<String>,

    #[serde(skip, default = "Vec::new")]
    pub exclude_patterns: Vec<Pattern>,

    #[serde(skip, default = "Vec::new")]
    pub include_patterns: Vec<Pattern>,

    #[serde(default = "get_default_loaders")]
    pub loaders: FxHashMap<String, serde_json::Value>,

    #[serde(default = "get_default_num_pls_per_dir")]
    pub num_pls_per_dir: u32,

    #[serde(default = "get_default_with_positions")]
    pub with_positions: bool,
}

impl Default for InfiIndexingConfig {
    fn default() -> Self {
        let mut indexing_config = InfiIndexingConfig {
            num_threads: get_default_num_threads(),
            num_docs_per_block: get_default_num_docs_per_block(),
            pl_limit: get_default_pl_limit(),
            pl_cache_threshold: get_default_pl_cache_threshold(),
            exclude: get_default_exclude_patterns(),
            include: Vec::new(),
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            loaders: get_default_loaders(),
            num_pls_per_dir: get_default_num_pls_per_dir(),
            with_positions: get_default_with_positions(),
        };

        indexing_config.init_patterns();
        indexing_config
    }
}

impl InfiIndexingConfig {
    pub fn get_loaders_from_config(&self) -> Vec<LoaderBoxed> {
        let mut loaders: Vec<LoaderBoxed> = Vec::new();

        for (key, value) in self.loaders.clone() {
            match key.as_str() {
                "HtmlLoader" => loaders.push(HtmlLoader::get_new_html_loader(value)),
                "CsvLoader" => loaders.push(CsvLoader::get_new_csv_loader(value)),
                "JsonLoader" => loaders.push(JsonLoader::get_new_json_loader(value)),
                "TxtLoader" => loaders.push(TxtLoader::get_new_txt_loader(value)),
                "PdfLoader" => loaders.push(PdfLoader::get_new_pdf_loader(value)),
                _ => panic!("Unknown loader type encountered in config"),
            }
        }

        loaders
    }

    pub fn is_excluded(&self, relative_path: &Path) -> bool {
        self.exclude_patterns.iter().any(|pat| pat.matches_path(relative_path))
        ||
        !(
            self.include_patterns.is_empty()
            || self.include_patterns.iter().any(|pat| pat.matches_path(relative_path))
        )
    }

    fn init_patterns(&mut self) {
        self.exclude_patterns = self.exclude
            .iter()
            .map(|pat_str| Pattern::new(pat_str).expect("Invalid exclude glob pattern!"))
            .collect();

        self.include_patterns = self.include
            .iter()
            .map(|pat_str| Pattern::new(pat_str).expect("Invalid include glob pattern!"))
            .collect();
    }
}

#[derive(Deserialize)]
pub struct InfiConfig {
    #[serde(default = "get_default_preset")]
    pub preset: String,
    #[serde(default)]
    pub fields_config: FieldsConfig,
    #[serde(default)]
    pub lang_config: InfiLanguageConfig,
    #[serde(default)]
    pub indexing_config: InfiIndexingConfig,
    #[serde(skip)]
    pub json_config: Value,
}

impl InfiConfig {
    pub fn new(raw_config: String) -> Self {
        let mut config: InfiConfig = serde_json::from_str(&raw_config)
            .expect("infi_search.json does not match schema!");
        let json_config: Value = serde_json::from_str(&raw_config)
            .expect("infi_search.json does not match schema!");

        config.fields_config.merge_default_fields();

        match config.preset.as_str() {
            "small" => {
                preset_small::apply_config(&mut config, &json_config);
            },
            "medium" => {
                preset_medium::apply_config(&mut config, &json_config);
            },
            "large" => {
                preset_large::apply_config(&mut config, &json_config);
            },
            _ => {
                // ignore invalid presets
            }
        }

        config.json_config = json_config;
        config.indexing_config.init_patterns();

        config
    }
}

impl Default for InfiConfig {
    fn default() -> Self {
        InfiConfig::new("{}".to_owned())
    }
}
