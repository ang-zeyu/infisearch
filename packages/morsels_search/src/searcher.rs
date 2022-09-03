pub mod query;
pub mod query_parser;
pub mod query_preprocessor;
pub mod query_processor;
pub mod query_retriever;
mod futures;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use morsels_common::MetadataReader;
use morsels_common::MorselsLanguageConfigOpts;

use wasm_bindgen::prelude::wasm_bindgen;
#[cfg(feature = "perf")]
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::dictionary::Dictionary;
use crate::docinfo::DocInfo;
use crate::postings_list_cache::PostingsListCache;

#[cfg(feature = "lang_ascii")]
use morsels_lang_ascii::ascii;
#[cfg(feature = "lang_latin")]
use morsels_lang_latin::latin;
#[cfg(feature = "lang_chinese")]
use morsels_lang_chinese::chinese;

use morsels_common::tokenize::SearchTokenizer;
use morsels_common::MorselsLanguageConfig;
use query_parser::{parse_query, QueryPartType};

struct SearcherConfig {
    indexing_config: IndexingConfig,
    lang_config: MorselsLanguageConfig,
    field_infos: Vec<FieldInfo>,
    num_scored_fields: usize,
    searcher_options: SearcherOptions,
}

struct IndexingConfig {
    num_pls_per_dir: u32,
    with_positions: bool,
}

struct FieldInfo {
    name: String,
    weight: f32,
    k: f32,
    b: f32,
}

struct SearcherOptions {
    url: String,
    number_of_expanded_terms: usize,
    pub use_query_term_proximity: bool,
    pl_lazy_cache_threshold: u32,
    result_limit: Option<u32>,
}

#[wasm_bindgen]
pub struct Searcher {
    dictionary: Dictionary,
    tokenizer: Box<dyn SearchTokenizer>,
    doc_info: DocInfo,
    searcher_config: SearcherConfig,
    invalidation_vector: Vec<u8>,
    postings_list_cache: PostingsListCache,

    // For soft dismax scoring
    num_scored_fields_less_one: f32,
}

#[cfg(feature = "lang_ascii")]
fn get_tokenizer(lang_config: &MorselsLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(ascii::new_with_options(lang_config))
}

#[cfg(feature = "lang_latin")]
fn get_tokenizer(lang_config: &MorselsLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(latin::new_with_options(lang_config))
}

#[cfg(feature = "lang_chinese")]
fn get_tokenizer(lang_config: &MorselsLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(chinese::new_with_options(lang_config))
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
#[wasm_bindgen]
pub fn get_new_searcher(
    metadata_buf: JsValue,
    num_pls_per_dir: u32,
    with_positions: bool,
    lang: String,
    stop_words: JsValue,  // serialized in workerSearcher.ts
    ignore_stop_words: Option<bool>,
    stemmer: Option<String>,
    max_term_len: Option<usize>,
    field_infos_raw: JsValue, // custom uint8array, serialized in workerSearcher.ts
    num_scored_fields: usize,
    url: String,
    number_of_expanded_terms: usize,
    use_query_term_proximity: bool,
    pl_lazy_cache_threshold: u32,
    result_limit: Option<u32>,
) -> Searcher {
    #[cfg(feature = "perf")]
    let window: web_sys::Window = js_sys::global().unchecked_into();
    #[cfg(feature = "perf")]
    let performance = window.performance().unwrap();
    #[cfg(feature = "perf")]
    let start = performance.now();

    let field_infos_raw = js_sys::Uint8Array::new(&field_infos_raw).to_vec();
    let mut field_infos = Vec::new();
    let mut field_infos_raw_pos = 0;
    while field_infos_raw_pos < field_infos_raw.len() {
        let name_len = field_infos_raw[field_infos_raw_pos] as usize;
        field_infos_raw_pos += 1;

        let name = unsafe {
            std::str::from_utf8_unchecked(&field_infos_raw[field_infos_raw_pos..field_infos_raw_pos + name_len])
        }.to_owned();
        field_infos_raw_pos += name_len;

        let weight = LittleEndian::read_f32(&field_infos_raw[field_infos_raw_pos..]);
        field_infos_raw_pos += 4;

        let k = LittleEndian::read_f32(&field_infos_raw[field_infos_raw_pos..]);
        field_infos_raw_pos += 4;

        let b = LittleEndian::read_f32(&field_infos_raw[field_infos_raw_pos..]);
        field_infos_raw_pos += 4;

        field_infos.push(FieldInfo { name, weight, k, b })
    }

    let stop_words = if stop_words.is_undefined() {
        None
    } else {
        let stop_words_raw = js_sys::Uint8Array::new(&stop_words).to_vec();
        let mut stop_words_vec = Vec::new();

        let mut i = 0;
        while i < stop_words_raw.len() {
            let len = stop_words_raw[i] as usize;
            i += 1;
            stop_words_vec.push(unsafe {
                std::str::from_utf8_unchecked(&stop_words_raw[i..i + len])
            }.to_owned());
            i += len;
        }

        Some(stop_words_vec)
    };

    let searcher_config = SearcherConfig {
        indexing_config: IndexingConfig {
            num_pls_per_dir,
            with_positions,
        },
        lang_config: MorselsLanguageConfig {
            lang,
            options: MorselsLanguageConfigOpts {
                stop_words,
                ignore_stop_words,
                stemmer,
                max_term_len,
            },
        },
        field_infos,
        num_scored_fields,
        searcher_options: SearcherOptions {
            url,
            number_of_expanded_terms,
            use_query_term_proximity,
            pl_lazy_cache_threshold,
            result_limit,
        }
    };

    let mut metadata_rdr = MetadataReader::new(
        js_sys::Uint8Array::new(&metadata_buf).to_vec()
    );

    let mut invalidation_vector = Vec::new();
    metadata_rdr.get_invalidation_vec(&mut invalidation_vector);

    let doc_info = DocInfo::create(&mut metadata_rdr, searcher_config.num_scored_fields);

    let tokenizer = get_tokenizer(&searcher_config.lang_config);

    let dictionary = metadata_rdr.setup_dictionary();

    #[cfg(feature = "perf")]
    {
        web_sys::console::log_1(&format!("Finished reading metadata.").into());
        web_sys::console::log_1(
            &format!("Dictionary initial setup took {}, num terms {}",
            performance.now() - start, dictionary.term_infos.len(),
        ).into());
        web_sys::console::log_1(
            &format!("Setup took {}", performance.now() - start).into(),
        );
    }

    let num_scored_fields_less_one = if searcher_config.num_scored_fields <= 1 {
        1.0
    } else {
        (searcher_config.num_scored_fields - 1) as f32
    };

    Searcher {
        dictionary,
        tokenizer,
        doc_info,
        searcher_config,
        invalidation_vector,
        postings_list_cache: PostingsListCache::new(),
        num_scored_fields_less_one
    }
}

#[wasm_bindgen]
impl Searcher {
    pub fn get_ptr(&self) -> *const Searcher {
        self
    }
}

#[allow(dead_code)]
#[wasm_bindgen]
pub async fn get_query(searcher: *mut Searcher, query: String) -> Result<query::Query, JsValue> {
    #[cfg(feature = "perf")]
    let window: web_sys::Window = js_sys::global().unchecked_into();
    #[cfg(feature = "perf")]
    let performance = window.performance().unwrap();
    #[cfg(feature = "perf")]
    let start = performance.now();

    let searcher_val = unsafe { &mut *searcher };
    let mut query_parts = parse_query(
        query,
        &*searcher_val.tokenizer,
        &searcher_val.searcher_config.field_infos.iter().map(|fi| fi.name.as_str()).collect(),
        searcher_val.searcher_config.indexing_config.with_positions,
        &searcher_val.dictionary,
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

    if is_free_text_query {
        searcher_val.remove_free_text_sw(&mut query_parts);
    }
    searcher_val.expand_term_postings_lists(&mut query_parts);

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Preprocess took {}, is_free_text_query {}", performance.now() - start, is_free_text_query).into());

    let term_pls = searcher_val.retrieve_term_pls(&mut query_parts).await;

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Population took {}", performance.now() - start).into());

    let result_heap = searcher_val.process_and_rank(&mut query_parts, &term_pls);

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Process took {}", performance.now() - start).into());

    let result_limit = searcher_val.searcher_config.searcher_options.result_limit;
    let query = searcher_val.create_query(query_parts, result_heap, result_limit);

    Ok(query)
}

#[cfg(test)]
pub mod test {
    use std::collections::BTreeMap;

    use morsels_common::{MorselsLanguageConfig, MorselsLanguageConfigOpts};
    use morsels_lang_ascii::ascii;

    use super::{FieldInfo, IndexingConfig, Searcher, SearcherConfig, SearcherOptions};
    use crate::dictionary::Dictionary;
    use crate::docinfo::DocInfo;
    use crate::postings_list_cache::PostingsListCache;

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
            tokenizer: Box::new(ascii::new_with_options(&MorselsLanguageConfig {
                lang: "ascii".to_owned(),
                options: MorselsLanguageConfigOpts::default(),
            })),
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
                    options: MorselsLanguageConfigOpts::default(),
                },
                field_infos,
                num_scored_fields: num_fields,
                searcher_options: SearcherOptions {
                    url: "/".to_owned(),
                    number_of_expanded_terms: 0,
                    use_query_term_proximity: true,
                    pl_lazy_cache_threshold: 0,
                    result_limit: None,
                },
            },
            invalidation_vector: vec![0; num_docs],
            postings_list_cache: PostingsListCache::new(),
            num_scored_fields_less_one: 1.0
        }
    }
}
