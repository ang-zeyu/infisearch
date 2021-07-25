pub mod query_parser;
pub mod query_preprocessor;
pub mod query_retriever;
pub mod query_processor;
pub mod query_postprocessor;
pub mod Query;

use crate::tokenize::english::EnglishTokenizer;
use crate::tokenize::Tokenizer;
use crate::Searcher::query_parser::QueryPart;
use crate::docinfo::DocInfo;
use std::collections::HashSet;

use query_parser::QueryPartType;
use query_parser::parse_query;
use crate::dictionary::Dictionary;
use crate::dictionary::setup_dictionary;

use serde::{Deserialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{wasm_bindgen};

#[derive(Deserialize)]
pub struct FieldInfo {
  pub id: u8,
  pub name: String,
  pub do_store: bool,
  pub weight: f32,
  pub k: f32,
  pub b: f32,
}

#[derive(Deserialize)]
struct SearcherOptions {
    url: String,
    use_query_term_expansion: bool,
    pub use_query_term_proximity: bool,
}

#[wasm_bindgen]
pub struct Searcher {
    base_url: String,
    dictionary: Dictionary,
    tokenizer: Box<dyn Tokenizer>,
    num_scored_fields: usize,
    field_infos: Vec<FieldInfo>,
    doc_info: DocInfo,
    searcher_options: SearcherOptions,
}

#[wasm_bindgen]
pub async fn get_new_searcher(
    base_url: String,
    num_scored_fields: usize,
    field_infos_js: JsValue,
    searcher_options: JsValue,
) -> Result<Searcher, JsValue> {
  let doc_info = DocInfo::create(base_url.clone(), num_scored_fields).await?;

  let dictionary = setup_dictionary(base_url.clone(), doc_info.num_docs).await?;

  let field_infos: Vec<FieldInfo> = field_infos_js.into_serde().unwrap();
  let searcher_options: SearcherOptions = searcher_options.into_serde().unwrap();

  Ok(Searcher {
    base_url,
    dictionary,
    tokenizer: Box::new(EnglishTokenizer::default()),
    num_scored_fields,
    field_infos,
    doc_info,
    searcher_options,
  })
}

#[wasm_bindgen]
impl Searcher {
  pub fn get_ptr(&self) -> *const Searcher {
    self
  }
}

fn get_aggregated_terms(query_parts: &Vec<QueryPart>, seen: &mut HashSet<String>, result: &mut Vec<String>) {
  for query_part in query_parts {
    if let Some(terms) = &query_part.terms {
      if query_part.isStopWordRemoved {
        result.push(query_part.originalTerms.as_ref().unwrap()[0].clone());
      }

      for term in terms {
        if seen.contains(term) {
          continue;
        }
        seen.insert(term.clone());
        result.push(term.clone());
      }
    } else if let Some(children) = &query_part.children {
      get_aggregated_terms(children, seen, result);
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

  web_sys::console::log_1(&JsValue::from_serde(&query_parts).unwrap());

  let is_free_text_query = query_parts.iter().all(|query_part| if let QueryPartType::TERM = query_part.partType { true } else { false });

  searcher_val.preprocess(&mut query_parts, is_free_text_query);

  web_sys::console::log_1(&format!("Preprocess took {}, is_free_text_query {}", performance.now() - start, is_free_text_query).into());

  let term_pls = searcher_val.populate_term_pls(&mut query_parts).await?;

  web_sys::console::log_1(&format!("Population took {}", performance.now() - start).into());

  let mut pls = searcher_val.process(&mut query_parts, term_pls);

  /* for pl in pls.iter() {
    web_sys::console::log_1(&format!("num term docs {} {}",
      if let Some(term) = pl.term.as_ref() { term } else { "" },
      pl.term_docs.len(),
    ).into());
  } */
  web_sys::console::log_1(&format!("Process took {}", performance.now() - start).into());

  searcher_val.postprocess(&mut query_parts, &mut pls).await?;

  web_sys::console::log_1(&format!("Post process took {}", performance.now() - start).into());

  let mut aggregated_terms: Vec<String> = Vec::new();
  get_aggregated_terms(&query_parts, &mut HashSet::new(), &mut aggregated_terms);
  
  let query = searcher_val.create_query(10, aggregated_terms, query_parts, pls, is_free_text_query);
  
  web_sys::console::log_1(&format!("Ranking took {}", performance.now() - start).into());

  Ok(query)
}
