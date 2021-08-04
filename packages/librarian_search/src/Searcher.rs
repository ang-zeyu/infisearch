pub mod query_parser;
pub mod query_preprocessor;
pub mod query_retriever;
pub mod query_processor;
pub mod Query;

use std::collections::HashSet;

use serde::{Deserialize};
use smartstring::alias::String as SmartString;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{wasm_bindgen};

use crate::docinfo::DocInfo;
use crate::dictionary::Dictionary;
use crate::dictionary::setup_dictionary;
use crate::PostingsListFileCache::PostingsListFileCache;

#[cfg(feature = "lang_latin")]
use librarian_lang_latin::english;
#[cfg(feature = "lang_chinese")]
use librarian_lang_chinese::chinese;

use librarian_common::LibrarianLanguageConfig;
use librarian_common::tokenize::Tokenizer;
use query_parser::{QueryPart, QueryPartType, parse_query};

#[derive(Deserialize)]
struct IndexingConfig {
  #[serde(rename = "withPositions")]
  with_positions: bool,
  #[serde(rename = "plNamesToCache")]
  pl_names_to_cache: Vec<u32>
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
    #[serde(rename = "useQueryTermExpansion")]
    use_query_term_expansion: bool,
    #[serde(rename = "useQueryTermProximity")]
    pub use_query_term_proximity: bool,
}

#[wasm_bindgen]
pub struct Searcher {
    base_url: String,
    dictionary: Dictionary,
    tokenizer: Box<dyn Tokenizer>,
    num_scored_fields: usize,
    field_infos: Vec<FieldInfo>,
    indexing_config: IndexingConfig,
    doc_info: DocInfo,
    searcher_options: SearcherOptions,
    pl_file_cache: PostingsListFileCache,
}

#[cfg(feature = "lang_latin")]
fn get_tokenizer(language_config: LibrarianLanguageConfig) -> Box<dyn Tokenizer> {
  if let Some(options) = language_config.options {
    Box::new(english::new_with_options(serde_json::from_value(options).unwrap()))
  } else {
    Box::new(english::EnglishTokenizer::default())
  }
}

#[cfg(feature = "lang_chinese")]
fn get_tokenizer(language_config: LibrarianLanguageConfig) -> Box<dyn Tokenizer> {
  if let Some(options) = language_config.options {
    Box::new(chinese::new_with_options(serde_json::from_value(options).unwrap()))
  } else {
    Box::new(chinese::ChineseTokenizer::default())
  }
}

#[wasm_bindgen]
pub async fn get_new_searcher(
    base_url: String,
    num_scored_fields: usize,
    field_infos_js: JsValue,
    indexing_config: JsValue,
    language_config: JsValue,
    searcher_options: JsValue,
) -> Result<Searcher, JsValue> {
  let doc_info = DocInfo::create(base_url.clone(), num_scored_fields).await?;
  
  let language_config: LibrarianLanguageConfig = language_config.into_serde().unwrap();
  let tokenizer = get_tokenizer(language_config);
  let build_trigram = tokenizer.use_default_trigram();

  let dictionary = setup_dictionary(SmartString::from(&base_url), doc_info.num_docs, build_trigram).await?;

  let field_infos: Vec<FieldInfo> = field_infos_js.into_serde().unwrap();
  let indexing_config: IndexingConfig = indexing_config.into_serde().unwrap();
  let searcher_options: SearcherOptions = searcher_options.into_serde().unwrap();

  let pl_file_cache = PostingsListFileCache::create(&base_url, &indexing_config.pl_names_to_cache).await;

  Ok(Searcher {
    base_url,
    dictionary,
    tokenizer,
    num_scored_fields,
    field_infos,
    indexing_config,
    doc_info,
    searcher_options,
    pl_file_cache,
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
pub async fn get_query(searcher: *const Searcher, query: String) -> Result<Query::Query, JsValue> {
  
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
