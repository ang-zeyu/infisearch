use std::rc::Rc;
use std::collections::HashSet;

use regex::Regex;
use rustc_hash::FxHashMap;
use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize};
use smartstring::alias::String as SmartString;

use librarian_common::tokenize::TermInfo;
use librarian_common::tokenize::SearchTokenizeResult;
use librarian_common::tokenize::Tokenizer;
use crate::ascii_folding_filter;

lazy_static! {
  static ref TERM_FILTER: Regex = Regex::new(r#"(^\W+)|(\W+$)|([\[\](){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.])"#).unwrap();
  static ref SENTENCE_SPLITTER: Regex = Regex::new(r#"[.?!](\s+|$)"#).unwrap();
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
  stemmer: Stemmer
}

impl Default for EnglishTokenizer {
  fn default() -> EnglishTokenizer {
    EnglishTokenizer {
      stop_words: get_stop_words_set(get_default_stop_words()),
      use_stemmer: false,
      stemmer: Stemmer::create(Algorithm::English),
    }
  }
}

#[derive(Deserialize)]
pub struct EnglishTokenizerOptions {
  stop_words: Option<Vec<String>>,
  stemmer: Option<String>,
}

pub fn new_with_options(options: EnglishTokenizerOptions) -> EnglishTokenizer {
  let stop_words = if let Some(stop_words) = options.stop_words {
    get_stop_words_set(stop_words)
  } else {
    get_stop_words_set(get_default_stop_words())
  };

  let use_stemmer = options.stemmer.is_some();

  let stemmer = if let Some(stemmer_lang) = options.stemmer {
    match stemmer_lang.as_str() {
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
  }
}

impl Tokenizer for EnglishTokenizer {
  fn tokenize(&self, mut text: String) -> Vec<String> {
    text.make_ascii_lowercase();
    SENTENCE_SPLITTER
      .split(&text)
      .flat_map(|sent_slice| sent_slice.split_whitespace()
        .map(|term_slice| {
          let folded = ascii_folding_filter::to_ascii(term_slice);
          let filtered = TERM_FILTER.replace_all(&folded, "");

          if self.use_stemmer { self.stemmer.stem(&folded).into_owned() } else { filtered.into_owned() }
        })
        .filter(|term| {
          let term_byte_len = term.as_bytes().len();
          term_byte_len > 0 && term_byte_len <= 120
        })
      )
      .collect()
  }

  fn wasm_tokenize(&self, text: String) -> SearchTokenizeResult {
    let should_expand = !text.ends_with(" ");
    SearchTokenizeResult {
      terms: self.tokenize(text),
      should_expand,
    }
  }

  fn is_stop_word(&self, term: &str) -> bool {
    return self.stop_words.contains(term);
  }

  fn use_default_trigram(&self) -> bool {
    return true;
  }

  fn get_best_corrected_term(&self, _term: &String, _dictionary: &FxHashMap<Rc<SmartString>, Rc<TermInfo>>) -> Option<String> {
    return None;
  }

  fn get_expanded_terms(&self, _term: &String, _dictionary: &FxHashMap<Rc<SmartString>, Rc<TermInfo>>) -> FxHashMap<String, f32> {
    return FxHashMap::default();
  }
}
