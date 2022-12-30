use super::InfiConfig;

use serde_json::Value;

pub fn apply_preset_override(
    config: &mut InfiConfig,
    json_config: &Value,
    num_docs_per_store: u32,
    cache_all_field_stores: bool,
    pl_limit: u32,
    pl_cache_threshold: u32
) {
    if let Some(val) = json_config.get("fields_config") {
        if val.get("num_docs_per_store").is_none() {
            config.fields_config.num_docs_per_store = num_docs_per_store;
        }

        if val.get("cache_all_field_stores").is_none() {
            config.fields_config.cache_all_field_stores = cache_all_field_stores;
        }
    } else {
        config.fields_config.num_docs_per_store = num_docs_per_store;
        config.fields_config.cache_all_field_stores = cache_all_field_stores;
    };

    if let Some(val) = json_config.get("indexing_config") {
        if val.get("pl_limit").is_none() {
            config.indexing_config.pl_limit = pl_limit;
        }

        if val.get("pl_cache_threshold").is_none() {
            config.indexing_config.pl_cache_threshold = pl_cache_threshold;
        }
    } else {
        config.indexing_config.pl_limit = pl_limit;
        config.indexing_config.pl_cache_threshold = pl_cache_threshold;
    };
}