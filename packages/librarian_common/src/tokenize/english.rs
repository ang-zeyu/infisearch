mod asciiFoldingFilter;

use crate::tokenize::TermInfo;
use std::rc::Rc;
use std::collections::HashSet;

use regex::Regex;
use rustc_hash::FxHashMap;

use crate::tokenize::Tokenizer;
use crate::tokenize::SearchTokenizeResult;

lazy_static! {
  static ref PUNCTUATION_FILTER: Regex = Regex::new(r#"[\[\](){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.]"#).unwrap();
  static ref BOUNDARY_FILTER: Regex = Regex::new(r#"(^\W)|(\W$)"#).unwrap();
  static ref SENTENCE_SPLITTER: Regex = Regex::new(r#"[.?!](\s+|$)"#).unwrap();
}

pub fn tokenize (mut text: String) -> Vec<String> {
  text.make_ascii_lowercase();
  SENTENCE_SPLITTER
    .split(&text)
    .flat_map(|sent_slice| sent_slice.split_whitespace()
      .map(|term_slice| {
        asciiFoldingFilter::to_ascii(&BOUNDARY_FILTER.replace_all(
          &PUNCTUATION_FILTER.replace_all(term_slice, ""), ""
        ))
      })
      .filter(|term| {
        let term_byte_len = term.as_bytes().len();
        term_byte_len > 0 && term_byte_len <= 120
      })
    )
    .collect()
}

pub fn wasm_tokenize(text: String) -> SearchTokenizeResult {
  let should_expand = !text.ends_with(" ");
  return SearchTokenizeResult {
    terms: tokenize(text),
    should_expand,
  }
}

pub fn get_stop_words() -> &'static str {
  // from tantivy
  r#"[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into",
    "is", "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then",
    "there", "these", "they", "this", "to", "was", "will", "with"
  ]"#
}

pub fn get_stop_words_set() -> HashSet<String> {
  let mut set: HashSet<String> = HashSet::default();
  let stop_words = vec![
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into",
    "is", "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then",
    "there", "these", "they", "this", "to", "was", "will", "with"
  ];
  for word in stop_words {
    set.insert(word.to_owned());
  }

  set
}

pub struct EnglishTokenizer {
  pub stop_words: HashSet<String>,
}

impl Default for EnglishTokenizer {
  fn default() -> EnglishTokenizer {
    EnglishTokenizer {
      stop_words: get_stop_words_set(),
    }
  }
}

impl Tokenizer for EnglishTokenizer {
  fn tokenize(&self, text: String) -> Vec<String> {
    return tokenize(text);
  }

  fn wasm_tokenize(&self, text: String) -> SearchTokenizeResult {
    return wasm_tokenize(text);
  }

  fn is_stop_word(&self, term: &str) -> bool {
    return self.stop_words.contains(term);
  }

  fn use_default_trigram(&self) -> bool {
    return true;
  }

  fn get_best_corrected_term(&self, _term: &String, _dictionary: &FxHashMap<Rc<String>, Rc<TermInfo>>) -> Option<String> {
    return None;
  }

  fn get_expanded_terms(&self, _term: &String, _dictionary: &FxHashMap<Rc<String>, Rc<TermInfo>>) -> FxHashMap<String, f32> {
    return FxHashMap::default();
  }
}
