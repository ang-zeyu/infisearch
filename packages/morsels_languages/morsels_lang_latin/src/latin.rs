use std::borrow::Cow;
use std::collections::{HashSet, BTreeMap};

use miniserde::json::Value;
use rust_stemmers::{Algorithm, Stemmer};
use rustc_hash::FxHashMap;
use smartstring::alias::String as SmartString;

use morsels_common::MorselsLanguageConfig;
#[cfg(feature = "indexer")]
use morsels_common::tokenize::IndexerTokenizer;
use morsels_common::tokenize::{TermInfo, SearchTokenizeResult, SearchTokenizer};
#[cfg(feature = "indexer")]
use morsels_lang_ascii::ascii_folding_filter;
use morsels_lang_ascii::ascii::ascii_and_nonword_filter;
#[cfg(feature = "indexer")]
use morsels_lang_ascii::ascii::SENTENCE_SPLITTER;
use morsels_lang_ascii::options;
use morsels_lang_ascii::stop_words::{get_stop_words_set, get_default_stop_words_set};
#[cfg(feature = "indexer")]
use morsels_lang_ascii::utils::term_filter;

pub struct Tokenizer {
    pub stop_words: HashSet<String>,
    #[cfg(feature = "indexer")]
    ignore_stop_words: bool,
    stemmer: Stemmer,
    max_term_len: usize,
}

pub struct TokenizerOptions {
    pub stop_words: Option<Vec<String>>,
    #[cfg(feature = "indexer")]
    ignore_stop_words: bool,
    pub stemmer: Option<String>,
    pub max_term_len: usize,
}

pub fn new_with_options(lang_config: &MorselsLanguageConfig) -> Tokenizer {
    let options = if let Value::Object(obj) = &lang_config.options {
        obj
    } else {
        panic!("language config options should be object");
    };

    let stop_words = if let Some(stop_words) = options::get_stop_words(&options) {
        get_stop_words_set(stop_words)
    } else {
        get_default_stop_words_set()
    };

    let stemmer = options.get("stemmer")
        .map_or(
            Stemmer::create(Algorithm::English),
            |v| if let Value::String(stemmer_lang) = v {
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
            }
        );

    let max_term_len = options::get_max_term_len(&options).min(250);

    Tokenizer {
        stop_words,
        #[cfg(feature = "indexer")]
        ignore_stop_words: options::get_ignore_stop_words(&options),
        stemmer,
        max_term_len,
    }
}

#[cfg(feature = "indexer")]
impl IndexerTokenizer for Tokenizer {
    fn tokenize<'a>(&self, text: &'a mut str) -> Vec<Vec<Cow<'a, str>>> {
        text.make_ascii_lowercase();
        SENTENCE_SPLITTER.split(text)
            .map(|sent_slice| {
                sent_slice
                    .split_whitespace()
                    .map(|term_slice| term_filter(ascii_folding_filter::to_ascii(term_slice)))
                    .filter(|term_slice| !(self.ignore_stop_words && self.stop_words.contains(term_slice.as_ref())))
                    .map(|term_slice| {
                        if let Cow::Owned(v) = self.stemmer.stem(&term_slice) {
                            Cow::Owned(v)
                        } else {
                            term_slice
                        }
                    })
                    .filter(|term| {
                        let term_byte_len = term.len();
                        term_byte_len > 0 && term_byte_len <= self.max_term_len
                    })
                    .collect()
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
                let preprocessed = ascii_and_nonword_filter(&mut terms, term_slice);

                let stemmed = if let Cow::Owned(v) = self.stemmer.stem(&preprocessed) {
                    terms.push(v.clone());
                    Cow::Owned(v)
                } else {
                    preprocessed
                };

                terms_searched.push(terms);

                stemmed
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
