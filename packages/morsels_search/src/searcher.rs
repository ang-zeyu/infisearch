pub mod query_parser;
pub mod query_preprocessor;
pub mod query_retriever;
pub mod query_processor;
pub mod query;

use std::collections::HashSet;

use serde::{Deserialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{wasm_bindgen};

use crate::docinfo::DocInfo;
use crate::dictionary::Dictionary;
use crate::dictionary::setup_dictionary;
use crate::postings_list_file_cache::PostingsListFileCache;

#[cfg(feature = "lang_latin")]
use morsels_lang_latin::english;
#[cfg(feature = "lang_chinese")]
use morsels_lang_chinese::chinese;

use morsels_common::MorselsLanguageConfig;
use morsels_common::tokenize::Tokenizer;
use query_parser::{QueryPart, QueryPartType, parse_query};

#[derive(Deserialize)]
struct SearcherConfig {
  #[serde(rename = "indexingConfig")]
  indexing_config: IndexingConfig,
  language: MorselsLanguageConfig,
  #[serde(rename = "fieldInfos")]
  field_infos: Vec<FieldInfo>,
  #[serde(rename = "numScoredFields")]
  num_scored_fields: usize,
  #[serde(rename = "searcherOptions")]
  searcher_options: SearcherOptions,
}

#[derive(Deserialize)]
struct IndexingConfig {
  #[serde(rename = "plNamesToCache")]
  pl_names_to_cache: Vec<u32>,
  #[serde(rename = "numPlsPerDir")]
  num_pls_per_dir: u32,
  #[serde(rename = "withPositions")]
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
struct SearcherOptions {
    url: String,
    #[serde(rename = "numberOfExpandedTerms")]
    number_of_expanded_terms: usize,
    #[serde(rename = "useQueryTermProximity")]
    pub use_query_term_proximity: bool,
}

#[wasm_bindgen]
pub struct Searcher {
  dictionary: Dictionary,
  tokenizer: Box<dyn Tokenizer>,
  doc_info: DocInfo,
  pl_file_cache: PostingsListFileCache,
  searcher_config: SearcherConfig,
}

#[cfg(feature = "lang_latin")]
fn get_tokenizer(language_config: &mut MorselsLanguageConfig) -> Box<dyn Tokenizer> {
  if let Some(options) = &mut language_config.options {
    Box::new(english::new_with_options(serde_json::from_value(std::mem::take(options)).unwrap()))
  } else {
    Box::new(english::EnglishTokenizer::default())
  }
}

#[cfg(feature = "lang_chinese")]
fn get_tokenizer(language_config: &mut MorselsLanguageConfig) -> Box<dyn Tokenizer> {
  if let Some(options) = &mut language_config.options {
    Box::new(chinese::new_with_options(serde_json::from_value(std::mem::take(options)).unwrap()))
  } else {
    Box::new(chinese::ChineseTokenizer::default())
  }
}

#[wasm_bindgen]
pub async fn get_new_searcher(config_js: JsValue) -> Result<Searcher, JsValue> {
  let mut searcher_config: SearcherConfig = config_js.into_serde().expect("Morsels config does not match schema");
  let doc_info = DocInfo::create(
    &searcher_config.searcher_options.url,
    searcher_config.num_scored_fields
  ).await?;

  let tokenizer = get_tokenizer(&mut searcher_config.language);
  let build_trigram = tokenizer.use_default_trigram();

  let dictionary = setup_dictionary(
    &searcher_config.searcher_options.url,
    doc_info.num_docs,
    build_trigram,
  ).await?;

  let pl_file_cache = PostingsListFileCache::create(
    &searcher_config.searcher_options.url,
    &searcher_config.indexing_config.pl_names_to_cache,
    searcher_config.indexing_config.num_pls_per_dir
  ).await;

  Ok(Searcher {
    dictionary,
    tokenizer,
    doc_info,
    pl_file_cache,
    searcher_config,
  })
}

#[wasm_bindgen]
impl Searcher {
  pub fn get_ptr(&self) -> *const Searcher {
    self
  }
}

fn get_searched_terms(query_parts: &Vec<QueryPart>, seen: &mut HashSet<String>, result: &mut Vec<String>) {
  for query_part in query_parts {
    if let Some(terms) = &query_part.terms {
      if query_part.is_stop_word_removed {
        result.push(query_part.original_terms.as_ref().unwrap()[0].clone());
      }

      for term in terms {
        if seen.contains(term) {
          continue;
        }
        seen.insert(term.clone());
        result.push(term.clone());
      }
    } else if let Some(children) = &query_part.children {
      get_searched_terms(children, seen, result);
    }
  }
}

#[wasm_bindgen]
pub async fn get_query(searcher: *const Searcher, query: String) -> Result<query::Query, JsValue> {
  
  let window: web_sys::Window = js_sys::global().unchecked_into();
  let performance = window.performance().unwrap();
  let start = performance.now();

  let searcher_val = unsafe { &*searcher };
  let mut query_parts = parse_query(query, &searcher_val.tokenizer)?;
  
  web_sys::console::log_1(&format!("parse query took {}", performance.now() - start).into());

  let is_free_text_query = query_parts.iter().all(|query_part| if let QueryPartType::TERM = query_part.part_type {
    query_part.field_name.is_none()
  } else {
    false
  });

  searcher_val.preprocess(&mut query_parts, is_free_text_query);

  web_sys::console::log_1(&format!("Preprocess took {}, is_free_text_query {}", performance.now() - start, is_free_text_query).into());

  let term_pls = searcher_val.populate_term_pls(&mut query_parts).await?;

  web_sys::console::log_1(&format!("Population took {}", performance.now() - start).into());

  let pls = searcher_val.process(&mut query_parts, term_pls);

  /* for pl in pls.iter() {
    web_sys::console::log_1(&format!("num term docs {} {}",
      if let Some(term) = pl.term.as_ref() { term } else { "" },
      pl.term_docs.len(),
    ).into());
  } */
  web_sys::console::log_1(&format!("Process took {}", performance.now() - start).into());

  let mut searched_terms: Vec<String> = Vec::new();
  get_searched_terms(&query_parts, &mut HashSet::new(), &mut searched_terms);
  
  let query = searcher_val.create_query(10, searched_terms, query_parts, pls, is_free_text_query);
  
  web_sys::console::log_1(&format!("Ranking took {}", performance.now() - start).into());

  Ok(query)
}
