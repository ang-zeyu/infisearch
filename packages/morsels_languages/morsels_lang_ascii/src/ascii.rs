use std::borrow::Cow;
use std::collections::HashSet;
use std::rc::Rc;

use regex::Regex;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use smartstring::alias::String as SmartString;

use crate::ascii_folding_filter;
use crate::stop_words::{get_stop_words_set, get_default_stop_words_set};
use crate::utils::term_filter;
use morsels_common::tokenize::SearchTokenizeResult;
use morsels_common::tokenize::TermInfo;
use morsels_common::tokenize::Tokenizer as TokenizerTrait;

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

impl Tokenizer {
    #[inline(always)]
    fn tokenize_slice<'a>(&self, slice: &'a str) -> Vec<Cow<'a, str>> {
        let iter = slice
            .split_whitespace()
            .map(|term_slice| {
                let ascii_folded = ascii_folding_filter::to_ascii(term_slice);
                term_filter(ascii_folded)
            })
            .filter(|term| {
                let term_byte_len = term.len();
                term_byte_len > 0 && term_byte_len <= self.max_term_len
            });

        if self.ignore_stop_words {
            return iter.filter(|term| !self.stop_words.contains(term.as_ref())).collect();
        }

        iter.collect()
    }
}

impl TokenizerTrait for Tokenizer {
    fn tokenize<'a>(&self, text: &'a mut str) -> Vec<Vec<Cow<'a, str>>> {
        text.make_ascii_lowercase();
        SENTENCE_SPLITTER.split(text).map(|sent_slice| self.tokenize_slice(sent_slice)).collect()
    }

    fn wasm_tokenize(&self, mut text: String) -> SearchTokenizeResult {
        text.make_ascii_lowercase();
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

    fn get_best_corrected_term(
        &self,
        _term: &str,
        _dictionary: &FxHashMap<Rc<SmartString>, TermInfo>,
    ) -> Option<String> {
        None
    }

    fn get_expanded_terms(
        &self,
        _number_of_expanded_terms: usize,
        _term: &str,
        _dictionary: &FxHashMap<Rc<SmartString>, TermInfo>,
    ) -> FxHashMap<String, f32> {
        FxHashMap::default()
    }
}
