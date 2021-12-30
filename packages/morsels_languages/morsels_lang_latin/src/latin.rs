use std::borrow::Cow;
use std::collections::HashSet;
use std::rc::Rc;

use rust_stemmers::{Algorithm, Stemmer};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use smartstring::alias::String as SmartString;

use morsels_common::tokenize::SearchTokenizeResult;
use morsels_common::tokenize::TermInfo;
use morsels_common::tokenize::Tokenizer as TokenizerTrait;
use morsels_lang_ascii::ascii_folding_filter;
use morsels_lang_ascii::ascii::SENTENCE_SPLITTER;
use morsels_lang_ascii::stop_words::{get_stop_words_set, get_default_stop_words_set};
use morsels_lang_ascii::utils::term_filter;

pub struct Tokenizer {
    pub stop_words: HashSet<String>,
    ignore_stop_words: bool,
    stemmer: Stemmer,
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
            stop_words: get_default_stop_words_set(),
            ignore_stop_words: get_default_ignore_stop_words(),
            stemmer: Stemmer::create(Algorithm::English),
            max_term_len: get_default_max_term_len(),
        }
    }
}

#[derive(Deserialize)]
pub struct TokenizerOptions {
    pub stop_words: Option<Vec<String>>,
    #[serde(default="get_default_ignore_stop_words")]
    ignore_stop_words: bool,
    pub stemmer: Option<String>,
    #[serde(default = "get_default_max_term_len")]
    pub max_term_len: usize,
}

pub fn new_with_options(options: TokenizerOptions, for_search: bool) -> Tokenizer {
    let stop_words = if let Some(stop_words) = options.stop_words {
        get_stop_words_set(stop_words)
    } else {
        get_default_stop_words_set()
    };

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

    Tokenizer {
        stop_words,
        ignore_stop_words: if for_search { false } else { options.ignore_stop_words },
        stemmer,
        max_term_len: options.max_term_len
    }
}

impl Tokenizer {
    #[inline(always)]
    fn tokenize_slice<'a>(&self, slice: &'a str) -> Vec<Cow<'a, str>> {
        slice
            .split_ascii_whitespace()
            .map(|term_slice| term_filter(ascii_folding_filter::to_ascii(&term_slice)))
            .filter(|term_slice| !(self.ignore_stop_words && self.stop_words.contains(term_slice.as_ref())))
            .map(|term_slice| {
                if let Cow::Owned(v) = self.stemmer.stem(&term_slice) {
                    Cow::Owned(v)
                } else {
                    term_slice // unchanged
                }
            })
            .filter(|term| {
                let term_byte_len = term.len();
                term_byte_len > 0 && term_byte_len <= self.max_term_len
            })
            .collect()
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
