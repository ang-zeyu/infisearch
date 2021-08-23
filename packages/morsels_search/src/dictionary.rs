use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::rc::Rc;

use futures::join;
use rustc_hash::FxHashMap;
use smartstring::alias::String;
use strsim::levenshtein;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use morsels_common::dictionary::{self, DICTIONARY_TABLE_FILE_NAME, DICTIONARY_STRING_FILE_NAME};

pub type Dictionary = dictionary::Dictionary;

static TERM_EXPANSION_ALPHA: f32 = 0.85;
static SPELLING_CORRECTION_BASE_ALPHA: f32 = 0.6;


struct TermWeightPair(Rc<String>, f64);

impl Eq for TermWeightPair {}

impl PartialEq for TermWeightPair {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

impl Ord for TermWeightPair {
  fn cmp(&self, other: &Self) -> Ordering {
    if self.1 > other.1 {
      Ordering::Greater
    } else if self.1 < other.1 {
      Ordering::Less
    } else {
      Ordering::Equal
    }
  }
}

impl PartialOrd for TermWeightPair {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    if self.1 > other.1 {
      Some(Ordering::Greater)
    } else if self.1 < other.1 {
      Some(Ordering::Less)
    } else {
      Some(Ordering::Equal)
    }
  }
}

pub async fn setup_dictionary(url: &str, num_docs: u32, build_trigram: bool) -> Result<Dictionary, JsValue> {
  let window: web_sys::Window = js_sys::global().unchecked_into();

  /* let performance = window.performance().unwrap();
  let start = performance.now(); */

  let (table_resp_value, string_resp_value) = join!(
    JsFuture::from(window.fetch_with_str(&(url.to_owned() + "/" + DICTIONARY_TABLE_FILE_NAME))),
    JsFuture::from(window.fetch_with_str(&(url.to_owned() + "/" + DICTIONARY_STRING_FILE_NAME)))
  );

  let table_resp: Response = table_resp_value.unwrap().dyn_into().unwrap();
  let string_resp: Response = string_resp_value.unwrap().dyn_into().unwrap();
  let (table_array_buffer, string_array_buffer) = join!(
    JsFuture::from(table_resp.array_buffer()?),
    JsFuture::from(string_resp.array_buffer()?)
  );

  let table_vec = js_sys::Uint8Array::new(&table_array_buffer.unwrap()).to_vec();
  let string_vec = js_sys::Uint8Array::new(&string_array_buffer.unwrap()).to_vec();

  // web_sys::console::log_1(&format!("Dictionary table and string retrieval took {} {} {}", performance.now() - start, table_vec.len(), string_vec.len()).into());

  let dictionary = dictionary::setup_dictionary(table_vec, string_vec, num_docs, build_trigram);

  // web_sys::console::log_1(&format!("Dictionary initial setup took {}", performance.now() - start).into());

  Ok(dictionary)
}

pub trait SearchDictionary {
  fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String>;

  fn get_corrected_terms(&self, misspelled_term: &str) -> Vec<Rc<String>>;

  fn get_expanded_terms(&self, number_of_expanded_terms: usize, base_term: &str) -> FxHashMap<std::string::String, f32>;

  fn get_term_candidates(&self, base_term: &str) -> FxHashMap<Rc<String>, usize>;
}

impl SearchDictionary for Dictionary {
  fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String> {
    let mut best_term = Option::None;
    let mut min_idf = f64::MAX;
    for term in self.get_corrected_terms(misspelled_term) {
      let term_info = self.term_infos.get(&term).unwrap();
      if term_info.idf < min_idf {
        min_idf = term_info.idf;
        best_term = Option::from(term);
      }
    };

    if let Some(best_term) = best_term {
      let normal_string: std::string::String = std::string::String::from(&best_term[..]);
      Option::from(normal_string)
    } else {
      Option::None
    }
  }
  
  fn get_corrected_terms(&self, misspelled_term: &str) -> Vec<Rc<String>> {
    let levenshtein_candidates = self.get_term_candidates(misspelled_term);
    let base_term_char_count = misspelled_term.chars().count();
    let mut min_edit_distance_terms = Vec::new();
    let mut min_edit_distance = 3;

    for (term, score) in levenshtein_candidates {
      // (A intersect B) / (A union B)
      // For n-gram string, there are n - 2 tri-grams
      // Filter edit distance candidates by jacard coefficient first
      if ((score as f32) / ((term.chars().count() + base_term_char_count - 4 - score) as f32)) < SPELLING_CORRECTION_BASE_ALPHA {
        continue;
      }

      let edit_distance = levenshtein(&term, misspelled_term);
      if edit_distance >= 3 {
        continue;
      }

      if edit_distance < min_edit_distance {
        min_edit_distance_terms.clear();
        min_edit_distance_terms.push(term);
        min_edit_distance = edit_distance;
      } else if edit_distance == min_edit_distance {
        min_edit_distance_terms.push(term);
      }
    };

    return min_edit_distance_terms;
  }
  
  fn get_expanded_terms(&self, number_of_expanded_terms: usize, base_term: &str) -> FxHashMap<std::string::String, f32> {
    let mut expanded_terms: FxHashMap<std::string::String, f32> = FxHashMap::default();
    let base_term_char_count = base_term.chars().count();
    if base_term_char_count < 4 {
      return expanded_terms;
    }

    let prefix_check_candidates = self.get_term_candidates(base_term);
    let min_matching_trigrams = (TERM_EXPANSION_ALPHA * (base_term.chars().count() - 2) as f32).floor() as usize;

    let base_idf = if let Some(term_info) = self.term_infos.get(&String::from(base_term)) {
      term_info.idf
    } else {
      0.0
    };

    // number_of_expanded_terms terms with the closest idfs
    let mut top_n_min_heap: BinaryHeap<TermWeightPair> = BinaryHeap::with_capacity(number_of_expanded_terms);
    let mut max_idf_difference: f64 = 0.0;

    let min_baseterm_substring = &base_term[0..((TERM_EXPANSION_ALPHA * base_term_char_count as f32).floor() as usize)];
    for (term, score) in prefix_check_candidates {
      // Filter away candidates that quite match in terms of number of trigrams first
      if score < min_matching_trigrams {
        continue;
      }

      if term.starts_with(min_baseterm_substring) && &term[..] != base_term {
        let term_info = self.term_infos.get(&term).unwrap();
        let idf_difference = (term_info.idf - base_idf).abs();
        if idf_difference > max_idf_difference {
          max_idf_difference = idf_difference;
        }
        
        let idf = self.term_infos.get(&term).unwrap().idf;
        if top_n_min_heap.len() < number_of_expanded_terms {
          top_n_min_heap.push(TermWeightPair(term.into(), idf_difference));
        } else if idf < top_n_min_heap.peek().unwrap().1 {
          top_n_min_heap.pop();
          top_n_min_heap.push(TermWeightPair(term.into(), idf_difference));
        }
      }
    };

    for term_weight_pair in top_n_min_heap {
      let idf_proportion = term_weight_pair.1 / max_idf_difference;
      let weight = if idf_proportion > 0.3 { 0.3 } else { idf_proportion };
      expanded_terms.insert(std::string::String::from(&term_weight_pair.0[..]), weight as f32);
    }

    return expanded_terms;
  }
  
  fn get_term_candidates(&self, base_term: &str) -> FxHashMap<Rc<String>, usize> {
    let mut candidates: FxHashMap<Rc<String>, usize> = FxHashMap::default();
    for tri_gram in morsels_common::dictionary::trigrams::get_tri_grams(base_term) {
      match self.trigrams.get(tri_gram) {
        Some(terms) => {
          for term in terms {
            match candidates.get_mut(&**term) {
              Some(val) => {
                *val += 1;
              },
              None => {
                candidates.insert(Rc::clone(term), 1);
              }
            }
          }
        },
        None => {}
      }
    };

    return candidates;
  }
}
