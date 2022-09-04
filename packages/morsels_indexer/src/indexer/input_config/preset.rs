use super::MorselsConfig;

use serde_json::Value;

fn set_all_content_fields_do_store(config: &mut MorselsConfig, do_store: bool) {
    if !do_store {
        // Default value is already "false"
        return;
    }

    for field in config.fields_config.fields.iter_mut() {
        field.do_store = true;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_preset_override(
    config: &mut MorselsConfig,
    json_config: &Value,
    field_store_block_size: u32,
    cache_all_field_stores: bool,
    pl_limit: u32,
    pl_cache_threshold: u32,
    with_positions: bool,
    do_store_fields: bool,
    ignore_stop_words: bool
) {
    if let Some(val) = json_config.get("fields_config") {
        if val.get("field_store_block_size").is_none() {
            config.fields_config.field_store_block_size = field_store_block_size;
        }

        if val.get("cache_all_field_stores").is_none() {
            config.fields_config.cache_all_field_stores = cache_all_field_stores;
        }

        if val.get("fields").is_none() {
            set_all_content_fields_do_store(config, do_store_fields);
        }
    } else {
        config.fields_config.field_store_block_size = field_store_block_size;
        config.fields_config.cache_all_field_stores = cache_all_field_stores;
        set_all_content_fields_do_store(config, do_store_fields);
    };

    if config.lang_config.options.ignore_stop_words.is_none() {
        config.lang_config.options.ignore_stop_words = Some(ignore_stop_words);
    }

    if let Some(val) = json_config.get("indexing_config") {
        if val.get("pl_limit").is_none() {
            config.indexing_config.pl_limit = pl_limit;
        }

        if val.get("pl_cache_threshold").is_none() {
            config.indexing_config.pl_cache_threshold = pl_cache_threshold;
        }

        if val.get("with_positions").is_none() {
            config.indexing_config.with_positions = with_positions;
        }
    } else {
        config.indexing_config.pl_limit = pl_limit;
        config.indexing_config.pl_cache_threshold = pl_cache_threshold;
        config.indexing_config.with_positions = with_positions;
    };
}