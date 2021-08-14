use std::rc::Rc;
use std::collections::HashSet;

use jieba_rs::Jieba;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize};
use smartstring::alias::String as SmartString;

use morsels_common::tokenize::TermInfo;
use morsels_common::tokenize::SearchTokenizeResult;
use morsels_common::tokenize::Tokenizer;

lazy_static! {
  static ref PUNCTUATION_FILTER: Regex = Regex::new(r#"[\[\](){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.]"#).unwrap();
}

fn get_stop_words_set(stop_words_vec: Vec<String>) -> HashSet<String> {
  let mut set: HashSet<String> = HashSet::default();

  // From tantivy
  for word in stop_words_vec {
    set.insert(word.to_owned());
  }

  set
}

pub struct ChineseTokenizer {
  pub stop_words: HashSet<String>,
  jieba: Jieba,
}

impl Default for ChineseTokenizer {
  fn default() -> ChineseTokenizer {
    ChineseTokenizer {
      stop_words: get_stop_words_set(Vec::new()),
      jieba: Jieba::empty(),
    }
  }
}

#[derive(Deserialize)]
pub struct ChineseTokenizerOptions {
  stop_words: Option<Vec<String>>,
}

pub fn new_with_options(options: ChineseTokenizerOptions) -> ChineseTokenizer {
  let stop_words = if let Some(stop_words) = options.stop_words {
    get_stop_words_set(stop_words)
  } else {
    get_stop_words_set(Vec::new())
  };

  ChineseTokenizer {
    stop_words,
    jieba: Jieba::empty(),
  }
}

impl Tokenizer for ChineseTokenizer {
  fn tokenize(&self, mut text: String) -> Vec<String> {
    text.make_ascii_lowercase();
    self.jieba.cut(&text, false).into_iter()
      .map(|s| PUNCTUATION_FILTER.replace_all(s, "").into_owned())
      .filter(|s| !s.trim().is_empty())
      .collect()
  }

  fn wasm_tokenize(&self, mut text: String) -> SearchTokenizeResult {
    text.make_ascii_lowercase();

    let should_expand = !text.ends_with(' ');

    let terms = self.jieba.cut_for_search(&text, false).into_iter()
      .map(|s| PUNCTUATION_FILTER.replace_all(s, "").into_owned())
      .filter(|s| !s.trim().is_empty())
      .collect();

    SearchTokenizeResult {
      terms,
      should_expand,
    }
  }

  fn is_stop_word(&self, term: &str) -> bool {
    self.stop_words.contains(term)
  }

  fn use_default_trigram(&self) -> bool {
    false
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
