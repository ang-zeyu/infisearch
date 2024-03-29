pub mod query;
pub mod query_parser;
pub mod query_preprocessor;
pub mod query_processor;
pub mod query_retriever;
mod futures;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use infisearch_common::metadata::{EnumMax, MetadataReader};
use infisearch_common::language::InfiLanguageConfigOpts;

use infisearch_common::utils::push;
use wasm_bindgen::prelude::wasm_bindgen;
#[cfg(feature = "perf")]
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::dictionary::Dictionary;
use crate::doc_info::DocInfo;
use crate::postings_list_cache::PostingsListCache;
use crate::utils;

#[cfg(feature = "lang_ascii")]
use infisearch_lang_ascii::ascii;
#[cfg(feature = "lang_ascii_stemmer")]
use infisearch_lang_ascii_stemmer::ascii_stemmer;
#[cfg(feature = "lang_chinese")]
use infisearch_lang_chinese::chinese;

use infisearch_common::tokenize::SearchTokenizer;
use infisearch_common::language::InfiLanguageConfig;

struct SearcherConfig {
    indexing_config: IndexingConfig,
    lang_config: InfiLanguageConfig,
    field_infos: Vec<FieldInfo>,
    valid_fields: Vec<String>,
    num_scored_fields: usize,
    inner_url: String,
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
    max_auto_suffix_search_terms: usize,
    max_suffix_search_terms: usize,
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
fn get_tokenizer(lang_config: &InfiLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(ascii::new_with_options(lang_config))
}

#[cfg(feature = "lang_ascii_stemmer")]
fn get_tokenizer(lang_config: &InfiLanguageConfig) -> Box<dyn SearchTokenizer> {
    Box::new(ascii_stemmer::new_with_options(lang_config))
}

#[cfg(feature = "lang_chinese")]
fn get_tokenizer(lang_config: &InfiLanguageConfig) -> Box<dyn SearchTokenizer> {
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
    inner_url: String,
    max_auto_suffix_search_terms: usize,
    max_suffix_search_terms: usize,
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
    let mut valid_fields = Vec::new();
    let mut field_infos_raw_pos = 0;
    while field_infos_raw_pos < field_infos_raw.len() {
        let name_len = (unsafe { *field_infos_raw.get_unchecked(field_infos_raw_pos) }) as usize;
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

        if weight > 0.0 {
            valid_fields.push(name.clone());
        }
        field_infos.push(FieldInfo { name, weight, k, b });
    }
    utils::insertion_sort(&mut valid_fields, |a, b| a.len() > b.len());

    let stop_words = if stop_words.is_undefined() {
        None
    } else {
        let stop_words_raw = js_sys::Uint8Array::new(&stop_words).to_vec();
        let mut stop_words_vec = Vec::new();

        let mut i = 0;
        while i < stop_words_raw.len() {
            let len = (unsafe { *stop_words_raw.get_unchecked(i) }) as usize;
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
        lang_config: InfiLanguageConfig {
            lang,
            options: InfiLanguageConfigOpts {
                stop_words,
                ignore_stop_words,
                stemmer,
                max_term_len,
            },
        },
        field_infos,
        valid_fields,
        num_scored_fields,
        inner_url,
        searcher_options: SearcherOptions {
            url,
            max_auto_suffix_search_terms,
            max_suffix_search_terms,
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
pub async fn get_query(searcher: *mut Searcher, params_raw: JsValue) -> Result<query::Query, JsValue> {
    #[cfg(feature = "perf")]
    let window: web_sys::Window = js_sys::global().unchecked_into();
    #[cfg(feature = "perf")]
    let performance = window.performance().unwrap();
    #[cfg(feature = "perf")]
    let start = performance.now();

    // --------------------------------------------------------------------------
    // Parameter parsing

    let params_raw = js_sys::Uint8Array::new(&params_raw).to_vec();

    // -----------------------------------
    // Query

    let query_length = LittleEndian::read_u32(&params_raw) as usize;
    let mut params_raw_pos = 4 + query_length;
    let query_string = unsafe {
        std::str::from_utf8_unchecked(params_raw.get_unchecked(4..params_raw_pos)).to_owned()
    };

    // -----------------------------------
    // Enums

    // Format:
    // num enums (1 byte)
    //   enum id (1 byte)
    //   number of enum values for this enum (1 byte)
    //   enum value's internal ids (times number of enum values) (1 byte each)
    let num_enums = unsafe { *params_raw.get_unchecked(params_raw_pos) } as usize;
    params_raw_pos += 1;

    let mut enum_filters = Vec::with_capacity(num_enums);
    for _i in 0..num_enums {
        let enum_id = unsafe { *params_raw.get_unchecked(params_raw_pos) } as usize;
        params_raw_pos += 1;
        let num_ev_ids = unsafe { *params_raw.get_unchecked(params_raw_pos) };
        params_raw_pos += 1;

        let mut ev_ids = [false; EnumMax::MAX as usize];
        for _j in 0..num_ev_ids {
            unsafe {
                *ev_ids.get_unchecked_mut(*params_raw.get_unchecked(params_raw_pos) as usize) = true;
            }
            params_raw_pos += 1;
        }

        push::push_wo_grow(&mut enum_filters, (enum_id, ev_ids));
    }

    // -----------------------------------
    // I64 Min Max filters

    // Format:
    // num filters (1 byte)
    //   i64 id (1 byte)
    //   number of enum values for this enum (1 byte)
    //   enum value's internal ids (times number of enum values) (1 byte each)

    let num_i64_fields = unsafe { *params_raw.get_unchecked(params_raw_pos) } as usize;
    params_raw_pos += 1;

    let mut i64_filters = Vec::with_capacity(num_i64_fields);
    for _i in 0..num_i64_fields {
        let i64_id = unsafe { *params_raw.get_unchecked(params_raw_pos) } as usize;
        params_raw_pos += 1;

        let has_gte = unsafe { *params_raw.get_unchecked(params_raw_pos) } == 1;
        params_raw_pos += 1;
        let gte = if has_gte {
            let ret = Some(LittleEndian::read_i64(
                unsafe { params_raw.get_unchecked(params_raw_pos..) }
            ));
            params_raw_pos += 8;
            ret
        } else {
            None
        };

        let has_lte = unsafe { *params_raw.get_unchecked(params_raw_pos) } == 1;
        params_raw_pos += 1;
        let lte = if has_lte {
            let ret = Some(LittleEndian::read_i64(
                unsafe { params_raw.get_unchecked(params_raw_pos..) }
            ));
            params_raw_pos += 8;
            ret
        } else {
            None
        };

        push::push_wo_grow(&mut i64_filters, (i64_id, gte, lte));
    }

    // -----------------------------------
    // Sort parameters
    let has_sort = unsafe { *params_raw.get_unchecked(params_raw_pos) } == 1;
    params_raw_pos += 1;
    let number_sort = if has_sort {
        let number_sort = Some(unsafe { *params_raw.get_unchecked(params_raw_pos) } as usize);
        params_raw_pos += 1;
        number_sort
    } else {
        None
    };

    let reverse_sort = unsafe { *params_raw.get_unchecked(params_raw_pos) } == 1;

    // --------------------------------------------------------------------------

    let searcher_val = unsafe { &mut *searcher };
    let mut query_parts = query_parser::parse_query(
        query_string,
        &mut *searcher_val.tokenizer,
        &searcher_val.searcher_config.valid_fields,
        searcher_val.searcher_config.indexing_config.with_positions,
        &searcher_val.dictionary,
    );

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("parse query took {}", performance.now() - start).into());

    searcher_val.expand_term_postings_lists(&mut query_parts);

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Preprocess took {}", performance.now() - start).into());

    let term_pls = searcher_val.retrieve_term_pls(&mut query_parts).await;

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Population took {}", performance.now() - start).into());

    let result_heap = searcher_val.process_and_rank(
        &mut query_parts, &term_pls, enum_filters, i64_filters, number_sort, reverse_sort,
    );

    #[cfg(feature = "perf")]
    web_sys::console::log_1(&format!("Process took {}", performance.now() - start).into());

    let result_limit = searcher_val.searcher_config.searcher_options.result_limit;
    let query = searcher_val.create_query(query_parts, result_heap, result_limit);

    Ok(query)
}

#[cfg(test)]
pub mod test {
    use std::collections::BTreeMap;

    use infisearch_common::language::{InfiLanguageConfig, InfiLanguageConfigOpts};
    use infisearch_lang_ascii::ascii;

    use super::{FieldInfo, IndexingConfig, Searcher, SearcherConfig, SearcherOptions};
    use crate::dictionary::Dictionary;
    use crate::doc_info::DocInfo;
    use crate::postings_list_cache::PostingsListCache;

    pub fn create_searcher(num_docs: usize) -> Searcher {
        let field_names = ["title", "body", "heading"];
        let num_fields = field_names.len();
        let mut valid_fields = Vec::new();
        let mut field_infos = Vec::new();
        for field_name in field_names {
            valid_fields.push(field_name.to_owned());
            field_infos.push(FieldInfo {
                name: field_name.to_owned(),
                weight: 0.3,
                k: 1.2,
                b: 0.75,
            });
        }

        Searcher {
            dictionary: Dictionary { term_infos: BTreeMap::default() },
            tokenizer: Box::new(ascii::new_with_options(&InfiLanguageConfig {
                lang: "ascii".to_owned(),
                options: InfiLanguageConfigOpts::default(),
            })),
            doc_info: DocInfo {
                doc_length_factors: vec![1.0; num_docs * num_fields],
                doc_length_factors_len: num_docs as u32,
                doc_enum_vals: Vec::new(),
                doc_i64_vals: Vec::new(),
                num_docs: num_docs as u32,
                num_fields,
                num_enum_fields: 0,
                num_i64_fields: 0,
            },
            searcher_config: SearcherConfig {
                indexing_config: IndexingConfig {
                    num_pls_per_dir: 0,
                    with_positions: true,
                },
                lang_config: InfiLanguageConfig {
                    lang: "ascii_stemmer".to_owned(),
                    options: InfiLanguageConfigOpts::default(),
                },
                field_infos,
                valid_fields,
                num_scored_fields: num_fields,
                inner_url: "/1261235123/".to_owned(),
                searcher_options: SearcherOptions {
                    url: "/".to_owned(),
                    max_auto_suffix_search_terms: 0,
                    max_suffix_search_terms: 0,
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
