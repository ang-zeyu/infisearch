use std::borrow::Cow;
use std::collections::{HashSet, BTreeMap};

#[cfg(feature = "indexer")]
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use smartstring::alias::String as SmartString;

use crate::ascii_folding_filter;
use crate::stop_words::{get_stop_words_set, get_default_stop_words_set};
use crate::utils::term_filter;
use morsels_common::tokenize::{TermInfo, SearchTokenizeResult, IndexerTokenizer, SearchTokenizer};

#[cfg(feature = "indexer")]
lazy_static! {
    pub static ref SENTENCE_SPLITTER: Regex = Regex::new(r#"[.,;?!]\s+"#).unwrap();
}

pub struct Tokenizer {
    pub stop_words: HashSet<String>,
    ignore_stop_words: bool,
    max_term_len: usize,
}

fn get_default_max_term_len() -> usize {
    80
}

fn get_default_ignore_stop_words() -> bool {
    false
}

impl Default for Tokenizer {
    fn default() -> Tokenizer {
        Tokenizer {
            stop_words: crate::stop_words::get_default_stop_words_set(),
            ignore_stop_words: get_default_ignore_stop_words(),
            max_term_len: get_default_max_term_len(),
        }
    }
}

#[derive(Deserialize)]
pub struct TokenizerOptions {
    pub stop_words: Option<Vec<String>>,
    #[serde(default="get_default_ignore_stop_words")]
    pub ignore_stop_words: bool,
    #[serde(default = "get_default_max_term_len")]
    pub max_term_len: usize,
}

pub fn new_with_options(options: TokenizerOptions, for_search: bool) -> Tokenizer {
    let stop_words = if let Some(stop_words) = options.stop_words {
        get_stop_words_set(stop_words)
    } else {
        get_default_stop_words_set()
    };

    Tokenizer {
        stop_words,
        ignore_stop_words: if for_search { false } else { options.ignore_stop_words },
        max_term_len: options.max_term_len
    }
}

pub fn ascii_and_nonword_filter<'a>(base_term_terms: &mut Vec<String>, term_slice: &'a str) -> Cow<'a, str> {
    base_term_terms.push(term_slice.to_owned());

    let mut ascii_replaced = ascii_folding_filter::to_ascii(term_slice);
    if let Cow::Owned(inner) = ascii_replaced {
        base_term_terms.push(inner.clone());
        ascii_replaced = Cow::Owned(inner);
    }

    if ascii_replaced.contains('\'') {
        // Somewhat hardcoded fix for this common keyboard "issue"
        base_term_terms.push(ascii_replaced.replace("'", "’"));
    }

    let term_filtered = term_filter(ascii_replaced);
    if let Cow::Owned(inner) = term_filtered {
        if !inner.is_empty() {
            base_term_terms.push(inner.clone());
        }
        Cow::Owned(inner)
    } else {
        term_filtered
    }
}

#[cfg(feature = "indexer")]
impl IndexerTokenizer for Tokenizer {
    fn tokenize<'a>(&self, text: &'a mut str) -> Vec<Vec<Cow<'a, str>>> {
        text.make_ascii_lowercase();
        SENTENCE_SPLITTER.split(text)
            .map(|sent_slice| {
                let iter = sent_slice
                    .split_whitespace()
                    .map(|term_slice| term_filter(ascii_folding_filter::to_ascii(term_slice)))
                    .filter(|term| {
                        let term_byte_len = term.len();
                        term_byte_len > 0 && term_byte_len <= self.max_term_len
                    });
        
                if self.ignore_stop_words {
                    iter.filter(|term| !self.stop_words.contains(term.as_ref())).collect()
                } else {
                    iter.collect()
                }
            })
            .collect()
    }
}

impl SearchTokenizer for Tokenizer {
    fn search_tokenize(&self, mut text: String, terms_searched: &mut Vec<Vec<String>>) -> SearchTokenizeResult {
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let terms = text
            .split_whitespace()
            .map(|term_slice| {
                let mut terms = Vec::new();
                let filtered = ascii_and_nonword_filter(&mut terms, term_slice);
                terms_searched.push(terms);
                filtered
            })
            .filter(|term| {
                let term_byte_len = term.len();
                term_byte_len > 0 && term_byte_len <= self.max_term_len
            })
            .map(|cow| cow.into_owned())
            .collect();

        SearchTokenizeResult {
            terms,
            should_expand,
        }
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
