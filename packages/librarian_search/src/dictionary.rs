mod trigrams;

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::rc::Rc;

use byteorder::{ByteOrder, LittleEndian};
use futures::join;
use rustc_hash::FxHashMap;
use strsim::levenshtein;
use smartstring::alias::String;
use smartstring::alias::String as SmartString;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use trigrams::get_tri_grams;
use crate::utils::varint::decode_var_int;
use librarian_common::tokenize::TermInfo;

static CORRECTION_ALPHA: f32 = 0.85;
static SPELLING_CORRECTION_BASE_ALPHA: f32 = 0.625;


pub struct Dictionary {
    pub term_infos: FxHashMap<Rc<String>, Rc<TermInfo>>,
    trigrams: FxHashMap<SmartString, Vec<Rc<String>>>,
}

struct TermWeightPair(String, f64, f32);

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

pub async fn setup_dictionary(url: String, num_docs: u32, build_trigram: bool) -> Result<Dictionary, JsValue> {
  let window: web_sys::Window = js_sys::global().unchecked_into();

  let performance = window.performance().unwrap();
  let start = performance.now();

  /* let urls = format!("[\"{}/dictionaryTable\",\"{}/dictionaryString\"]", url, url);
  let ptrs: Vec<u32> = vec![0, 0];
  web_sys::console::log_1(&format!("urls {} {}", urls, ptrs.as_ptr() as u32).into());
  
  fetchMultipleArrayBuffers(urls, ptrs.as_ptr() as u32).await?;

  web_sys::console::log_1(&format!("ptrs {} {} took {}", ptrs[0], ptrs[1], performance.now() - start).into()); */


  let (table_resp_value, string_resp_value) = join!(
    JsFuture::from(window.fetch_with_str(&(url.clone() + "/dictionaryTable"))),
    JsFuture::from(window.fetch_with_str(&(url + "/dictionaryString")))
  );

  let table_resp: Response = table_resp_value.unwrap().dyn_into().unwrap();
  let string_resp: Response = string_resp_value.unwrap().dyn_into().unwrap();
  let (table_array_buffer, string_array_buffer) = join!(
    JsFuture::from(table_resp.array_buffer()?),
    JsFuture::from(string_resp.array_buffer()?)
  );

  let table_vec = js_sys::Uint8Array::new(&table_array_buffer.unwrap()).to_vec();
  let string_vec = js_sys::Uint8Array::new(&string_array_buffer.unwrap()).to_vec();

  web_sys::console::log_1(&format!("Dictionary table and string retrieval took {} {} {}", performance.now() - start, table_vec.len(), string_vec.len()).into());

  let mut term_infos: FxHashMap<Rc<String>, Rc<TermInfo>> = FxHashMap::default();

  let mut postings_file_name = 0;
  let mut dict_string_pos = 0;
  let mut dict_table_pos = 0;
  let mut prev_term: Rc<String> = Rc::new(SmartString::from(""));

  let table_vec_len = table_vec.len();
  while dict_table_pos < table_vec_len {
    let val_and_length = decode_var_int(&table_vec[dict_table_pos..]);
    dict_table_pos += val_and_length.1;

    // new postings list delimiter
    let doc_freq = val_and_length.0;
    if doc_freq == 0 {
      postings_file_name += 1;
      continue;
    }
    
    let postings_file_offset = LittleEndian::read_u16(&table_vec[dict_table_pos..]);
    dict_table_pos += 2;

    let prefix_len = string_vec[dict_string_pos] as usize;
    dict_string_pos += 1;

    let remaining_len = string_vec[dict_string_pos] as usize;
    dict_string_pos += 1;

    let term = Rc::new(
      SmartString::from(&prev_term[..prefix_len]) +
        unsafe { std::str::from_utf8_unchecked(&string_vec[dict_string_pos..dict_string_pos + remaining_len]) }
    );
    dict_string_pos += remaining_len;

    term_infos.insert(Rc::clone(&term), Rc::new(TermInfo {
      doc_freq,
      idf: (1.0 + (num_docs as f64 - doc_freq as f64 + 0.5) / (doc_freq as f64 + 0.5)).ln(),
      postings_file_name,
      postings_file_offset,
    }));

    prev_term = term;
  }

  web_sys::console::log_1(&format!("Dictionary initial setup took {}", performance.now() - start).into());

  let trigrams = if build_trigram { Dictionary::setup_trigrams(&term_infos) } else { FxHashMap::default() };

  web_sys::console::log_1(&format!("Dictionary trigram setup took {}", performance.now() - start).into());

  Ok(Dictionary {
    term_infos,
    trigrams,
  })
}

impl Dictionary {
  pub fn get_term_info(&self, term: &str) -> Option<&Rc<TermInfo>> {
    self.term_infos.get(&String::from(term))
  }

  fn setup_trigrams(term_infos: &FxHashMap<Rc<String>, Rc<TermInfo>>) -> FxHashMap<SmartString, Vec<Rc<String>>> {
    let mut trigrams: FxHashMap<SmartString, Vec<Rc<String>>> = FxHashMap::default();

    for term in term_infos.keys() {
      for term_trigram in get_tri_grams(term) {
        // web_sys::console::log_1(&format!("trigram {}", term_trigram).into());
        match trigrams.get_mut(term_trigram) {
          Some(terms) => {
            terms.push(Rc::clone(term));
          },
          None => {
            let mut term_vec: Vec<Rc<String>> = Vec::with_capacity(20);
            term_vec.push(Rc::clone(term));
            trigrams.insert(SmartString::from(term_trigram), term_vec);
          }
        }
      }
    }

    trigrams
  }
}

impl Dictionary {
  pub fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String> {
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
      let normal_string: std::string::String = best_term.into();
      Option::from(normal_string)
    } else {
      Option::None
    }
  }
  
  fn get_corrected_terms(&self, misspelled_term: &str) -> Vec<String> {
    let levenshtein_candidates = self.get_term_candidates(misspelled_term, true);
    let mut min_edit_distance_terms = Vec::new();
    let mut min_edit_distance = 3;

    for term in levenshtein_candidates {
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
  
  pub fn get_expanded_terms(&self, base_term: &str) -> FxHashMap<std::string::String, f32> {
    let mut expanded_terms: FxHashMap<std::string::String, f32> = FxHashMap::default();
    let base_term_char_count = base_term.chars().count();
    if base_term_char_count < 4 {
      return expanded_terms;
    }

    let prefix_check_candidates = self.get_term_candidates(base_term, false);

    let max_expanded_terms = 3; // TODO make amount configurable
    // 3 lowest idf (most common) terms
    let mut top_3_min_heap: BinaryHeap<TermWeightPair> = BinaryHeap::with_capacity(max_expanded_terms);

    let min_baseterm_substring = &base_term[0..((CORRECTION_ALPHA * base_term_char_count as f32).floor() as usize)];
    for term in prefix_check_candidates {
      if term.starts_with(min_baseterm_substring) && term != base_term {
        let score = 1.0 / ((term.chars().count() - min_baseterm_substring.chars().count() + 1) as f32);
        if score >= 0.2 {
          let idf = self.term_infos.get(&term).unwrap().idf;
          if top_3_min_heap.len() < max_expanded_terms {
            top_3_min_heap.push(TermWeightPair(term.into(), idf, score));
          } else if idf < top_3_min_heap.peek().unwrap().1 {
            top_3_min_heap.pop();
            top_3_min_heap.push(TermWeightPair(term.into(), idf, score));
          }
        }
      }
    };

    for term_weight_triple in top_3_min_heap {
      expanded_terms.insert(term_weight_triple.0.into(), term_weight_triple.2);
    }

    return expanded_terms;
  }
  
  fn get_term_candidates(&self, base_term: &str, use_jacard: bool) -> Vec<String> {
    let mut num_base_term_trigrams: usize = 0;

    let mut candidates: FxHashMap<String, usize> = FxHashMap::default();
    for tri_gram in get_tri_grams(base_term) {
      match self.trigrams.get(tri_gram) {
        Some(terms) => {
          for term in terms {
            match candidates.get_mut(&**term) {
              Some(val) => {
                *val += 1;
              },
              None => {
                candidates.insert((**term).to_owned(), 1);
              }
            }
          }
        },
        None => {}
      }

      num_base_term_trigrams += 1;
    };

    let min_matching_trigrams = (CORRECTION_ALPHA * num_base_term_trigrams as f32).floor();

    let base_term_char_count = base_term.chars().count();
    return candidates.into_iter()
      .filter(|(term, score)| {
        if use_jacard {
          // (A intersect B) / (A union B)
          // For n-gram string, there are n - 2 tri-grams
          ((*score as f32) / ((term.chars().count() + base_term_char_count - 4 - score) as f32))
          >= SPELLING_CORRECTION_BASE_ALPHA
        } else {
          (*score as f32) >= min_matching_trigrams
        }
      })
      .map(|(term, _score)| term)
      .collect();
  }
}
