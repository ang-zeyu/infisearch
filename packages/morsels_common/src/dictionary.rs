pub mod trigrams;

use std::rc::Rc;

use byteorder::{ByteOrder, LittleEndian};
use rustc_hash::FxHashMap;
use smartstring::alias::String;
use smartstring::alias::String as SmartString;

use trigrams::get_tri_grams;
use crate::tokenize::TermInfo;
use crate::utils::varint;

pub static DICTIONARY_TABLE_FILE_NAME: &str = "_dictionary_table";
pub static DICTIONARY_STRING_FILE_NAME: &str = "_dictionary_string";

pub struct Dictionary {
    pub term_infos: FxHashMap<Rc<String>, Rc<TermInfo>>,
    pub trigrams: FxHashMap<SmartString, Vec<Rc<String>>>,
}

#[inline(always)]
pub fn setup_dictionary(
    table_vec: Vec<u8>,
    string_vec: Vec<u8>,
    num_docs: u32,
    build_trigram: bool
) -> Dictionary {
  let mut term_infos: FxHashMap<Rc<String>, Rc<TermInfo>> = FxHashMap::default();

  let mut postings_file_name = 0;
  let mut dict_string_pos = 0;
  let mut dict_table_pos = 0;
  let mut prev_term: Rc<String> = Rc::new(SmartString::from(""));

  let table_vec_len = table_vec.len();
  while dict_table_pos < table_vec_len {
    let (doc_freq, doc_freq_len) = varint::decode_var_int(&table_vec[dict_table_pos..]);
    dict_table_pos += doc_freq_len;

    // new postings list delimiter
    if doc_freq == 0 {
      postings_file_name += 1;
      continue;
    }
    
    let postings_file_offset = LittleEndian::read_u32(&table_vec[dict_table_pos..]);
    dict_table_pos += 4;

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

  let trigrams = if build_trigram { Dictionary::setup_trigrams(&term_infos) } else { FxHashMap::default() };

  Dictionary {
    term_infos,
    trigrams,
  }
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
