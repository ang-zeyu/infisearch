use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use morsels_common::MorselsLanguageConfig;

use crate::MORSELS_VERSION;
use crate::fieldinfo::{FieldInfoOutput, EnumInfo};
use super::Indexer;

use serde::{Serialize, Deserialize};

// Separate struct to support serializing for --config-init option but not output config
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MorselsIndexingOutputConfig {
    pl_names_to_cache: Vec<u32>,
    num_docs_per_block: u32,
    num_pls_per_dir: u32,
    with_positions: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MorselsOutputConfig {
    ver: String,
    index_ver: String,
    last_doc_id: u32,
    indexing_config: MorselsIndexingOutputConfig,
    lang_config: MorselsLanguageConfig,
    cache_all_field_stores: bool,
    pub field_infos: Vec<FieldInfoOutput>,
    num_scored_fields: usize,
    num_docs_per_store: u32,
    num_stores_per_dir: u32,
}

pub fn write_output_config(indexer: Indexer, mut enums_ev_strs: Vec<Vec<String>>) {
    drop(indexer.doc_miner);

    // Add in the enum string values sorted according to their enum_id and ev_ids
    let mut field_infos = indexer.field_infos.to_output();
    for field_info in &mut field_infos {
        if let Some(EnumInfo { enum_id, enum_values }) = &mut field_info.enum_info {
            *enum_values = std::mem::take(&mut enums_ev_strs[*enum_id]);
        }
    }

    let serialized = serde_json::to_string(&MorselsOutputConfig {
        ver: MORSELS_VERSION.to_owned(),
        index_ver: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string(),
        last_doc_id: indexer.doc_id_counter,
        indexing_config: MorselsIndexingOutputConfig {
            pl_names_to_cache: indexer.incremental_info.pl_names_to_cache.clone(),
            num_docs_per_block: indexer.indexing_config.num_docs_per_block,
            num_pls_per_dir: indexer.indexing_config.num_pls_per_dir,
            with_positions: indexer.indexing_config.with_positions,
        },
        lang_config: indexer.lang_config.clone(),
        cache_all_field_stores: indexer.cache_all_field_stores,
        field_infos,
        num_scored_fields: indexer.field_infos.num_scored_fields,
        num_docs_per_store: indexer.field_infos.num_docs_per_store,
        num_stores_per_dir: indexer.field_infos.num_stores_per_dir,
    })
    .unwrap();

    File::create(indexer.output_folder_path.join("morsels_config.json"))
        .unwrap()
        .write_all(serialized.as_bytes())
        .unwrap();
}
