pub mod query;
pub mod query_parser;
pub mod query_preprocessor;
pub mod query_processor;
pub mod query_retriever;

use morsels_common::BitmapDocinfoDicttableReader;
use morsels_common::dictionary;
use serde::Deserialize;
use serde_json::Value;
use wasm_bindgen::prelude::wasm_bindgen;
#[cfg(feature = "perf")]
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::dictionary::Dictionary;
use crate::docinfo::DocInfo;

#[cfg(feature = "lang_ascii")]
use morsels_lang_ascii::ascii;
#[cfg(feature = "lang_latin")]
use morsels_lang_latin::latin;
#[cfg(feature = "lang_chinese")]
use morsels_lang_chinese::chinese;

use morsels_common::tokenize::SearchTokenizer;
use morsels_common::MorselsLanguageConfig;
use query_parser::{parse_query, QueryPart, QueryPartType};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearcherConfig {
    indexing_config: IndexingConfig,
    lang_config: MorselsLanguageConfig,
    field_infos: Vec<FieldInfo>,
    num_scored_fields: usize,
    searcher_options: SearcherOptions,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexingConfig {
    num_pls_per_dir: u32,
    with_positions: bool,
}

#[derive(Deserialize)]
struct FieldInfo {
    name: String,
    weight: f32,
    k: f32,
    b: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearcherOptions {
    url: String,
    number_of_expanded_terms: usize,
    pub use_query_term_proximity: bool,
    result_limit: Option<u32>,
}

#[wasm_bindgen]
pub struct Searcher {
    dictionary: Dictionary,
    tokenizer: Box<dyn SearchTokenizer>,
    doc_info: DocInfo,
    searcher_config: SearcherConfig,
    invalidation_vector: Vec<u8>,
    // For soft dismax scoring
    num_scored_fields_less_one: f32,
}

#[cfg(feature = "lang_ascii")]
fn get_tokenizer(lang_config: &mut MorselsLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(ascii::new_with_options(serde_json::from_value(Value::Object(std::mem::take(&mut lang_config.options))).unwrap()))
}

#[cfg(feature = "lang_latin")]
fn get_tokenizer(lang_config: &mut MorselsLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(latin::new_with_options(serde_json::from_value(Value::Object(std::mem::take(&mut lang_config.options))).unwrap()))
}

#[cfg(feature = "lang_chinese")]
fn get_tokenizer(lang_config: &mut MorselsLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(chinese::new_with_options(serde_json::from_value(Value::Object(std::mem::take(&mut lang_config.options))).unwrap()))
}

#[allow(dead_code)]
#[wasm_bindgen]
pub async fn get_new_searcher(
    config_js: JsValue,
    bitmap_docinfo_dt_buf: JsValue,
    dict_string_buf: JsValue,
) -> Result<Searcher, JsValue> {
    let mut searcher_config: SearcherConfig = serde_wasm_bindgen::from_value(config_js).expect("Morsels config does not match schema");

    #[cfg(feature = "perf")]
    let window: web_sys::Window = js_sys::global().unchecked_into();
    #[cfg(feature = "perf")]
    let performance = window.performance().unwrap();
    #[cfg(feature = "perf")]
    let start = performance.now();

    let bitmap_docinfo_dt = js_sys::Uint8Array::new(&bitmap_docinfo_dt_buf).to_vec();
    let mut bitmap_docinfo_dt_rdr = BitmapDocinfoDicttableReader { buf: bitmap_docinfo_dt, pos: 0 };

    let mut invalidation_vector = Vec::new();
    bitmap_docinfo_dt_rdr.read_invalidation_vec(&mut invalidation_vector);

    let doc_info = DocInfo::create(&mut bitmap_docinfo_dt_rdr, searcher_config.num_scored_fields);

    let tokenizer = get_tokenizer(&mut searcher_config.lang_config);

    let string_vec = js_sys::Uint8Array::new(&dict_string_buf).to_vec();

    let dictionary = dictionary::setup_dictionary(
        bitmap_docinfo_dt_rdr.get_dicttable_slice(), string_vec,
    );

    #[cfg(feature = "perf")]
    web_sys::console::log_1(
        &format!("Finished reading bitmap_docinfo_dt_rdr. Pos {} Len {}",
        bitmap_docinfo_dt_rdr.pos, bitmap_docinfo_dt_rdr.buf.len(),
    ).into());

    #[cfg(feature = "perf")]
    web_sys::console::log_1(
        &format!("Dictionary initial setup took {}, num terms {}",
        performance.now() - start, dictionary.term_infos.len(),
    ).into());

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Setup took {}", performance.now() - start).into());

    let num_scored_fields_less_one = if searcher_config.num_scored_fields <= 1 {
        1.0
    } else {
        (searcher_config.num_scored_fields - 1) as f32
    };

    Ok(Searcher {
        dictionary,
        tokenizer,
        doc_info,
        searcher_config,
        invalidation_vector,
        num_scored_fields_less_one
    })
}

#[wasm_bindgen]
impl Searcher {
    pub fn get_ptr(&self) -> *const Searcher {
        self
    }
}

fn add_processed_terms(query_parts: &[QueryPart], result: &mut Vec<Vec<String>>) {
    for query_part in query_parts {
        if let Some(terms) = &query_part.terms {
            if query_part.is_expanded || query_part.is_corrected {
                let mut searched_terms = Vec::new();
                for term in terms {
                    searched_terms.push(term.clone());
                }
                if !searched_terms.is_empty() {
                    result.push(searched_terms);
                }
            }
        } else if let Some(children) = &query_part.children {
            add_processed_terms(children, result);
        }
    }
}

#[allow(dead_code)]
#[wasm_bindgen]
pub async fn get_query(searcher: *const Searcher, query: String) -> Result<query::Query, JsValue> {
    #[cfg(feature = "perf")]
    let window: web_sys::Window = js_sys::global().unchecked_into();
    #[cfg(feature = "perf")]
    let performance = window.performance().unwrap();
    #[cfg(feature = "perf")]
    let start = performance.now();

    let searcher_val = unsafe { &*searcher };
    let (mut query_parts, mut terms_searched) = parse_query(
        query,
        &*searcher_val.tokenizer,
        &searcher_val.searcher_config.field_infos.iter().map(|fi| fi.name.as_str()).collect(),
        searcher_val.searcher_config.indexing_config.with_positions,
    );

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("parse query took {}", performance.now() - start).into());

    let is_free_text_query = query_parts.iter().all(|query_part| {
        if let QueryPartType::Term = query_part.part_type {
            query_part.field_name.is_none()
        } else {
            false
        }
    });

    searcher_val.preprocess(&mut query_parts, is_free_text_query);

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Preprocess took {}, is_free_text_query {}", performance.now() - start, is_free_text_query).into());

    let term_pls = searcher_val.populate_term_pls(&mut query_parts).await?;

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Population took {}", performance.now() - start).into());

    let pls = searcher_val.process(&mut query_parts, term_pls);

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Process took {}", performance.now() - start).into());

    add_processed_terms(&query_parts, &mut terms_searched);

    let result_limit = searcher_val.searcher_config.searcher_options.result_limit;
    let query = searcher_val.create_query(terms_searched, query_parts, pls, result_limit);

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Ranking took {}", performance.now() - start).into());

    Ok(query)
}

#[cfg(test)]
pub mod test {
    use std::collections::BTreeMap;

    use morsels_common::MorselsLanguageConfig;
    use morsels_lang_ascii::ascii;

    use super::{FieldInfo, IndexingConfig, Searcher, SearcherConfig, SearcherOptions};
    use crate::dictionary::Dictionary;
    use crate::docinfo::DocInfo;

    use serde_json::json;

    pub fn create_searcher(num_docs: usize, num_fields: usize) -> Searcher {
        let mut field_infos = Vec::new();
        for i in 0..num_fields {
            field_infos.push(FieldInfo {
                name: format!("field{}", i).to_owned(),
                weight: 0.3,
                k: 1.2,
                b: 0.75,
            });
        }

        Searcher {
            dictionary: Dictionary { term_infos: BTreeMap::default() },
            tokenizer: Box::new(ascii::new_with_options(serde_json::from_value(json!({})).unwrap())),
            doc_info: DocInfo {
                doc_length_factors: vec![1.0; num_docs * num_fields],
                doc_length_factors_len: num_docs as u32,
                num_docs: num_docs as u32,
                num_fields,
            },
            searcher_config: SearcherConfig {
                indexing_config: IndexingConfig {
                    num_pls_per_dir: 0,
                    with_positions: true,
                },
                lang_config: MorselsLanguageConfig {
                    lang: "latin".to_owned(),
                    options: serde_json::from_str("{}").unwrap(),
                },
                field_infos,
                num_scored_fields: num_fields,
                searcher_options: SearcherOptions {
                    url: "/".to_owned(),
                    number_of_expanded_terms: 0,
                    use_query_term_proximity: true,
                    result_limit: None,
                },
            },
            invalidation_vector: vec![0; num_docs],
            num_scored_fields_less_one: 1.0
        }
    }
}
