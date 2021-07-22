mod trigrams;

use std::rc::Rc;

use wasm_bindgen::JsValue;
use rustc_hash::FxHashMap;
use futures::join;
use strsim::levenshtein;
use trigrams::get_tri_grams;

use byteorder::{ByteOrder, LittleEndian};
use crate::utils::varint::decode_var_int;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, Response};

static PREFIX_FRONT_CODE: u8 = 123;     // {
static SUBSEQUENT_FRONT_CODE: u8 = 125; // }

static CORRECTION_ALPHA: f32 = 0.85;
static SPELLING_CORRECTION_BASE_ALPHA: f32 = 0.625;

pub struct TermInfo {
    pub doc_freq: u32,
    pub idf: f64,
    pub max_term_score: f32,
    pub postings_file_name: u32,
    pub postings_file_offset: u16,
}

pub struct Dictionary {
    pub term_infos: FxHashMap<Rc<String>, Rc<TermInfo>>,
    trigrams: FxHashMap<String, Vec<Rc<String>>>,
}

#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_namespace = self, catch)]
  async fn fetchMultipleArrayBuffers(urls: String, ptr: u32) -> Result<(), JsValue>;
}

pub async fn setup_dictionary(url: String, num_docs: u32) -> Result<Dictionary, JsValue> {
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
  let mut front_code_prefix: Option<String> = Option::None;

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

    let max_term_score = LittleEndian::read_f32(&table_vec[dict_table_pos..]);
    dict_table_pos += 4;

    let term_len = string_vec[dict_string_pos] as usize;
    dict_string_pos += 1;

    if let Some(prefix) = front_code_prefix.as_ref() {
      if string_vec[dict_string_pos] == SUBSEQUENT_FRONT_CODE {
        dict_string_pos += 1;

        let term = prefix.to_owned() + unsafe { std::str::from_utf8_unchecked(&string_vec[dict_string_pos..(dict_string_pos + term_len)]) };
        dict_string_pos += term_len;

        /* if term.find('{').is_some() || term.find('}').is_some() {
          return Err(JsValue::from(format!("Uh ohhz {} {}",
            if let Some(prefix) = front_code_prefix { prefix } else { "".to_owned() },
            term
          )));
        } */

        term_infos.insert(Rc::new(term), Rc::new(TermInfo {
          doc_freq,
          idf: (1.0 + (num_docs as f64 - doc_freq as f64 + 0.5) / (doc_freq as f64 + 0.5)).ln(),
          max_term_score,
          postings_file_name,
          postings_file_offset,
        }));

        continue;
      }
      
      front_code_prefix = Option::None;
    }

    // from_utf8_unchecked must be used here as the term may contain the PREFIX_FRONT_CODE
    let mut term = unsafe { std::str::from_utf8_unchecked(&string_vec[dict_string_pos..(dict_string_pos + term_len)]).to_owned() };
    dict_string_pos += term_len;

    if let Some(prefix_front_code_idx) = term.find('{') {
      front_code_prefix = Option::from(term[0..prefix_front_code_idx].to_owned());

      term.replace_range(prefix_front_code_idx..prefix_front_code_idx + 1, ""); // remove the '{'
      term.push_str(unsafe { std::str::from_utf8_unchecked(&[string_vec[dict_string_pos]]) });

      // Redecode the full string, then remove the '{'
      /* term = unsafe {
        std::str::from_utf8_unchecked(&string_vec[(dict_string_pos - term_len)..(dict_string_pos + 1)])
          .replace('{', "")
      }; */
      
      dict_string_pos += 1;
    } else if dict_string_pos < string_vec.len() && string_vec[dict_string_pos] == PREFIX_FRONT_CODE {
      front_code_prefix = Option::from(term.clone());
      dict_string_pos += 1;
    }

    /* if term.find('{').is_some() || term.find('}').is_some() {
      return Err(JsValue::from(format!("Uh ohhz {} {}",
        if let Some(prefix) = front_code_prefix { prefix } else { "".to_owned() },
        term
      )));
    } */

    term_infos.insert(Rc::new(term), Rc::new(TermInfo {
      doc_freq,
      idf: (1.0 + (num_docs as f64 - doc_freq as f64 + 0.5) / (doc_freq as f64 + 0.5)).ln(),
      max_term_score,
      postings_file_name,
      postings_file_offset,
    }));
  }

  web_sys::console::log_1(&format!("Dictionary initial setup took {}", performance.now() - start).into());

  let trigrams = Dictionary::setup_trigrams(&term_infos);

  web_sys::console::log_1(&format!("Dictionary trigram setup took {}", performance.now() - start).into());

  Ok(Dictionary {
    term_infos,
    trigrams,
  })
}

impl Dictionary {
  pub fn get_term_info(&self, term: &String) -> Option<&Rc<TermInfo>> {
    self.term_infos.get(term)
  }

  fn setup_trigrams(term_infos: &FxHashMap<Rc<String>, Rc<TermInfo>>) -> FxHashMap<String, Vec<Rc<String>>> {
    let mut trigrams: FxHashMap<String, Vec<Rc<String>>> = FxHashMap::default();

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
            trigrams.insert(term_trigram.to_owned(), term_vec);
          }
        }
      }
    }

    trigrams
  }
}

impl Dictionary {
  pub fn get_best_corrected_term(&self, misspelled_term: &String) -> Option<String> {
    let mut best_term = Option::None;
    let mut min_idf = f64::MAX;
    for term in self.get_corrected_terms(misspelled_term) {
      let term_info = self.term_infos.get(&term).unwrap();
      if term_info.idf < min_idf {
        min_idf = term_info.idf;
        best_term = Option::from(term);
      }
    };
    return best_term;
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
  
  pub fn get_expanded_terms(&self, base_term: &str) -> FxHashMap<String, f32> {
    let mut expanded_terms: FxHashMap<String, f32> = FxHashMap::default();
    let base_term_char_count = base_term.chars().count();
    if base_term_char_count < 4 {
      return expanded_terms;
    }

    let prefix_check_candidates = self.get_term_candidates(base_term, false);

    let min_baseterm_substring = &base_term[0..((CORRECTION_ALPHA * base_term_char_count as f32).floor() as usize)];
    for term in prefix_check_candidates {
      if term.starts_with(min_baseterm_substring) && term != base_term {
        let score = 1.0 / ((term.chars().count() - min_baseterm_substring.chars().count() + 1) as f32);
        if score >= 0.2 {
          expanded_terms.insert(term, score);
        }
      }
    };

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
