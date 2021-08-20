use std::borrow::Cow;
use std::rc::Rc;
use std::collections::HashSet;

use regex::Regex;
use rustc_hash::FxHashMap;
use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize};
use smartstring::alias::String as SmartString;

use morsels_common::tokenize::TermInfo;
use morsels_common::tokenize::SearchTokenizeResult;
use morsels_common::tokenize::Tokenizer;
use crate::ascii_folding_filter;

lazy_static! {
  static ref TERM_FILTER: Regex = Regex::new(r#"(^\W+)|(\W+$)|([\[\]\\(){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.])"#).unwrap();
  static ref SENTENCE_SPLITTER: Regex = Regex::new(r#"[.,;?!]\s+"#).unwrap();
}

fn get_default_stop_words() -> Vec<String> {
  vec![
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into",
    "is", "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then",
    "there", "these", "they", "this", "to", "was", "will", "with"
  ].into_iter()
    .map(|s| s.to_owned())
    .collect()
}

fn get_stop_words_set(stop_words_vec: Vec<String>) -> HashSet<String> {
  let mut set: HashSet<String> = HashSet::default();

  // From tantivy
  for word in stop_words_vec {
    set.insert(word.to_owned());
  }

  set
}

pub struct EnglishTokenizer {
  pub stop_words: HashSet<String>,
  use_stemmer: bool,
  stemmer: Stemmer,
  max_term_len: usize,
}

fn get_default_max_term_len() -> usize {
  80
}

impl Default for EnglishTokenizer {
  fn default() -> EnglishTokenizer {
    EnglishTokenizer {
      stop_words: get_stop_words_set(get_default_stop_words()),
      use_stemmer: false,
      stemmer: Stemmer::create(Algorithm::English),
      max_term_len: get_default_max_term_len(),
    }
  }
}

#[derive(Deserialize)]
pub struct EnglishTokenizerOptions {
  stop_words: Option<Vec<String>>,
  stemmer: Option<String>,
  #[serde(default = "get_default_max_term_len")]
  max_term_len: usize,
}

pub fn new_with_options(options: EnglishTokenizerOptions) -> EnglishTokenizer {
  let stop_words = if let Some(stop_words) = options.stop_words {
    get_stop_words_set(stop_words)
  } else {
    get_stop_words_set(get_default_stop_words())
  };

  let use_stemmer = options.stemmer.is_some();

  let stemmer = if let Some(stemmer_lang) = options.stemmer {
    match stemmer_lang.to_lowercase().as_str() {
      "arabic" => Stemmer::create(Algorithm::Arabic),
      "danish" => Stemmer::create(Algorithm::Danish),
      "dutch" => Stemmer::create(Algorithm::Dutch),
      "english" => Stemmer::create(Algorithm::English),
      "finnish" => Stemmer::create(Algorithm::Finnish),
      "french" => Stemmer::create(Algorithm::French),
      "german" => Stemmer::create(Algorithm::German),
      "greek" => Stemmer::create(Algorithm::Greek),
      "hungarian" => Stemmer::create(Algorithm::Hungarian),
      "italian" => Stemmer::create(Algorithm::Italian),
      "norwegian" => Stemmer::create(Algorithm::Norwegian),
      "portuguese" => Stemmer::create(Algorithm::Portuguese),
      "romanian" => Stemmer::create(Algorithm::Romanian),
      "russian" => Stemmer::create(Algorithm::Russian),
      "spanish" => Stemmer::create(Algorithm::Spanish),
      "swedish" => Stemmer::create(Algorithm::Swedish),
      "tamil" => Stemmer::create(Algorithm::Tamil),
      "turkish" => Stemmer::create(Algorithm::Turkish),
      _ => Stemmer::create(Algorithm::English),
    }
  } else {
    Stemmer::create(Algorithm::English)
  };

  EnglishTokenizer {
    stop_words,
    use_stemmer,
    stemmer,
    max_term_len: options.max_term_len,
  }
}

// Custom replace_all regex implementation accepting cow to make lifetimes comply
// See https://github.com/rust-lang/regex/issues/676
fn term_filter<'a>(input: Cow<'a, str>) -> Cow<'a, str> {
  let mut match_iter = TERM_FILTER.find_iter(&input);
  if let Some(first) = match_iter.next() {
      let mut output:Vec<u8> = Vec::with_capacity(input.len());
      output.extend_from_slice(input[..first.start()].as_bytes());
      let mut start = first.end();

      loop {
        if let Some(next) = match_iter.next() {
          output.extend_from_slice(input[start..next.start()].as_bytes());
          start = next.end();
        } else {
          output.extend_from_slice(input[start..].as_bytes());
          return Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
        }
      }
  } else {
      input
  }
}

impl EnglishTokenizer {
  #[inline(always)]
  fn tokenize_slice<'a> (&self, slice: &'a str) -> Vec<Cow<'a, str>> {
    slice.split_whitespace()
      .map(|term_slice| {
        let ascii_folded = ascii_folding_filter::to_ascii(&term_slice);
        let filtered = term_filter(ascii_folded); 
  
        if self.use_stemmer {
          if let Cow::Owned(v) = self.stemmer.stem(&filtered) {
            Cow::Owned(v)
          } else {
            filtered // unchanged
          }
        } else {
          filtered
        }
      })
      .filter(|term| {
        let term_byte_len = term.len();
        term_byte_len > 0 && term_byte_len <= self.max_term_len
      })
      .collect()
  }
}

impl Tokenizer for EnglishTokenizer {
  fn tokenize<'a> (&self, text: &'a mut str) -> Vec<Vec<Cow<'a, str>>> {
    text.make_ascii_lowercase();
    SENTENCE_SPLITTER
      .split(text)
      .map(|sent_slice| self.tokenize_slice(sent_slice))
      .collect()
  }

  fn wasm_tokenize(&self, text: String) -> SearchTokenizeResult {
    let should_expand = !text.ends_with(' ');
    SearchTokenizeResult {
      terms: self.tokenize_slice(&text).into_iter().map(|cow| cow.into_owned()).collect(),
      should_expand,
    }
  }

  fn is_stop_word(&self, term: &str) -> bool {
    self.stop_words.contains(term)
  }

  fn use_default_trigram(&self) -> bool {
    true
  }

  fn get_best_corrected_term(&self, _term: &str, _dictionary: &FxHashMap<Rc<SmartString>, Rc<TermInfo>>) -> Option<String> {
    None
  }

  fn get_expanded_terms(
    &self,
    _number_of_expanded_terms: usize,
    _term: &str,
    _dictionary: &FxHashMap<Rc<SmartString>, Rc<TermInfo>>,
  ) -> FxHashMap<String, f32> {
    FxHashMap::default()
  }
}
