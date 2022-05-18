use std::{borrow::Cow, collections::BTreeMap};
use std::collections::HashSet;

use jieba_rs::Jieba;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use smartstring::alias::String as SmartString;

use morsels_common::tokenize::{TermInfo, SearchTokenizeResult, IndexerTokenizer, SearchTokenizer};

lazy_static! {
    static ref PUNCTUATION_FILTER: Regex =
        Regex::new(r#"[\[\]\\(){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.]"#).unwrap();
}

fn get_stop_words_set(stop_words_vec: Vec<String>) -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::default();

    for word in stop_words_vec {
        set.insert(word.to_owned());
    }

    set
}

fn get_default_ignore_stop_words() -> bool {
    false
}

pub struct Tokenizer {
    pub stop_words: HashSet<String>,
    ignore_stop_words: bool,
    jieba: Jieba,
}

impl Default for Tokenizer {
    fn default() -> Tokenizer {
        Tokenizer {
            stop_words: get_stop_words_set(Vec::new()),
            ignore_stop_words: get_default_ignore_stop_words(),
            jieba: Jieba::empty(),
        }
    }
}

#[derive(Deserialize)]
pub struct TokenizerOptions {
    stop_words: Option<Vec<String>>,
    #[serde(default="get_default_ignore_stop_words")]
    ignore_stop_words: bool,
}

pub fn new_with_options(options: TokenizerOptions, for_search: bool) -> Tokenizer {
    let stop_words = if let Some(stop_words) = options.stop_words {
        get_stop_words_set(stop_words)
    } else {
        get_stop_words_set(Vec::new())
    };

    Tokenizer {
        stop_words,
        ignore_stop_words: if for_search { false } else { options.ignore_stop_words },
        jieba: Jieba::empty(),
    }
}

impl IndexerTokenizer for Tokenizer {
    fn tokenize<'a>(&self, text: &'a mut str) -> Vec<Vec<Cow<'a, str>>> {
        text.make_ascii_lowercase();
        self.jieba
            .cut(text, false)
            .into_iter()
            .filter(|cut| !cut.trim().is_empty())
            .map(|s| PUNCTUATION_FILTER.replace_all(s, ""))
            .fold(vec![Vec::new()], |mut acc, next| {
                if next.trim().is_empty() {
                    acc.push(Vec::new()); // Split on punctuation
                } else if !(self.ignore_stop_words && self.stop_words.contains(next.as_ref())) {
                    acc.last_mut().unwrap().push(next);
                }
                acc
            })
    }
}

impl SearchTokenizer for Tokenizer {
    fn search_tokenize(&self, mut text: String, terms_searched: &mut Vec<Vec<String>>) -> SearchTokenizeResult {
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let terms = self
            .jieba
            .cut_for_search(&text, false)
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let mut terms = vec![s.to_owned()];

                let replaced = PUNCTUATION_FILTER.replace_all(s, "");
                let filtered = if let Cow::Owned(replaced) = replaced {
                    if !replaced.trim().is_empty() {
                        terms.push(replaced.clone());
                    }
                    replaced
                } else {
                    replaced.into_owned()
                };

                terms_searched.push(terms);

                filtered
            })
            .filter(|s| !s.trim().is_empty())
            .collect();

        SearchTokenizeResult { terms, should_expand }
    }

    fn is_stop_word(&self, term: &str) -> bool {
        self.stop_words.contains(term)
    }

    fn use_default_fault_tolerance(&self) -> bool {
        true
    }

    fn get_best_corrected_term(
        &self,
        _term: &str,
        _dictionary: &BTreeMap<SmartString, TermInfo>,
    ) -> Option<String> {
        None
    }

    fn get_prefix_terms(
        &self,
        _number_of_expanded_terms: usize,
        _term: &str,
        _dictionary: &BTreeMap<SmartString, TermInfo>,
    ) -> FxHashMap<String, f32> {
        FxHashMap::default()
    }
}
