use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use morsels_common::MorselsLanguageConfig;

use crate::MORSELS_VERSION;
use crate::fieldinfo::FieldInfoOutput;
use crate::loader::Loader;
use super::Indexer;

use rustc_hash::FxHashMap;
use serde::Serialize;

// Separate struct to support serializing for --config-init option but not output config
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MorselsIndexingOutputConfig {
    loader_configs: FxHashMap<String, Box<dyn Loader>>,
    pl_names_to_cache: Vec<u32>,
    num_docs_per_block: u32,
    num_pls_per_dir: u32,
    with_positions: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MorselsOutputConfig<'a> {
    ver: &'static str,
    index_ver: String,
    last_doc_id: u32,
    indexing_config: MorselsIndexingOutputConfig,
    lang_config: &'a MorselsLanguageConfig,
    cache_all_field_stores: bool,
    field_infos: Vec<FieldInfoOutput>,
    num_scored_fields: usize,
    field_store_block_size: u32,
    num_stores_per_dir: u32,
}

pub fn write_output_config(indexer: &mut Indexer) {
    let serialized = serde_json::to_string(&MorselsOutputConfig {
        ver: MORSELS_VERSION,
        index_ver: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string(),
        last_doc_id: indexer.doc_id_counter,
        indexing_config: MorselsIndexingOutputConfig {
            loader_configs: std::mem::take(&mut indexer.loaders)
                .into_iter()
                .map(|loader| (loader.get_name(), loader))
                .collect(),
            pl_names_to_cache: indexer.incremental_info.pl_names_to_cache.clone(),
            num_docs_per_block: indexer.indexing_config.num_docs_per_block,
            num_pls_per_dir: indexer.indexing_config.num_pls_per_dir,
            with_positions: indexer.indexing_config.with_positions,
        },
        lang_config: &indexer.lang_config,
        cache_all_field_stores: indexer.cache_all_field_stores,
        field_infos: indexer.field_infos.to_output(),
        num_scored_fields: indexer.field_infos.num_scored_fields,
        field_store_block_size: indexer.field_infos.field_store_block_size,
        num_stores_per_dir: indexer.field_infos.num_stores_per_dir,
    })
    .unwrap();

    File::create(indexer.output_folder_path.join("morsels_config.json"))
        .unwrap()
        .write_all(serialized.as_bytes())
        .unwrap();
}
