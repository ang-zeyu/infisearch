use std::borrow::Cow;
#[cfg(feature = "indexer")]
use std::collections::HashSet;
use std::collections::BTreeMap;

#[cfg(feature = "indexer")]
use regex::Regex;
use smartstring::alias::String as SmartString;

use crate::ascii_folding_filter;
use crate::stop_words::get_stop_words;
use crate::utils::{term_filter, split_terms};
use morsels_common::MorselsLanguageConfig;
#[cfg(feature = "indexer")]
use morsels_common::tokenize::{IndexerTokenizer, TermIter};
use morsels_common::tokenize::{TermInfo, SearchTokenizeResult, SearchTokenizer};

#[cfg(feature = "indexer")]
lazy_static! {
    pub static ref SENTENCE_SPLITTER: Regex = Regex::new(r#"[.,;?!]\s+"#).unwrap();
}

pub struct Tokenizer {
    // Remove HashSet from the search binary, where speed benefits are minimal
    #[cfg(feature = "indexer")]
    pub stop_words: HashSet<String>,
    #[cfg(not(feature = "indexer"))]
    pub stop_words: Vec<String>,

    ignore_stop_words: bool,

    // Just needs to be filtered during indexing
    #[cfg(feature = "indexer")]
    max_term_len: usize,
}

pub fn new_with_options(lang_config: &MorselsLanguageConfig) -> Tokenizer {
    let stop_words = get_stop_words(lang_config, &[
        // Same list from tantivy
        "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is", "it", "no",
        "not", "of", "on", "or", "such", "that", "the", "their", "then", "there", "these", "they", "this",
        "to", "was", "will", "with"
    ]);

    #[cfg(feature = "indexer")]
    let max_term_len = lang_config.options.max_term_len.unwrap_or(80).min(250);

    Tokenizer {
        stop_words,
        ignore_stop_words: lang_config.options.ignore_stop_words.unwrap_or(false),
        #[cfg(feature = "indexer")]
        max_term_len,
    }
}

pub fn ascii_and_nonword_filter<'a>(term_inflections: &mut Vec<String>, term_slice: &'a str) -> Cow<'a, str> {
    term_inflections.push(term_slice.to_owned());

    let mut ascii_replaced = ascii_folding_filter::to_ascii(term_slice);
    if let Cow::Owned(ascii_replaced_inner) = ascii_replaced {
        if !ascii_replaced_inner.is_empty() {
            term_inflections.push(ascii_replaced_inner.clone());
        }
        ascii_replaced = Cow::Owned(ascii_replaced_inner);
    }

    if ascii_replaced.contains('\'') {
        // Somewhat hardcoded fix for this common keyboard "issue"
        term_inflections.push(ascii_replaced.replace('\'', "’"));
    }

    let term_filtered = term_filter(ascii_replaced);
    if let Cow::Owned(inner) = term_filtered {
        if !inner.trim().is_empty() {
            term_inflections.push(inner.clone());
        }
        Cow::Owned(inner)
    } else {
        term_filtered
    }
}

#[cfg(feature = "indexer")]
impl IndexerTokenizer for Tokenizer {
    fn tokenize<'a>(&'a self, text: &'a mut str) -> TermIter<'a> {
        text.make_ascii_lowercase();
        let it = SENTENCE_SPLITTER.split(text)
            .flat_map(move |sent_slice| {
                sent_slice.split(split_terms)
                    .filter(|&s| !s.is_empty())
                    .map(|term_slice| term_filter(ascii_folding_filter::to_ascii(term_slice)))
                    .filter(move |term| {
                        let term_byte_len = term.len();
                        term_byte_len > 0
                            && term_byte_len <= self.max_term_len
                            && !(self.ignore_stop_words && self.stop_words.contains(term.as_ref()))
                    })
                    .map(Some).chain(std::iter::once(None))
            });

        Box::new(it)
    }
}

impl SearchTokenizer for Tokenizer {
    fn search_tokenize(&self, mut text: String) -> SearchTokenizeResult {
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let terms = text
            .split(split_terms)
            .filter_map(|term_slice| {
                if term_slice.is_empty() {
                    return None;
                }

                let mut term_inflections = Vec::new();

                let preprocessed = ascii_and_nonword_filter(&mut term_inflections, term_slice);

                if preprocessed.is_empty() {
                    return None;
                }

                if self.ignore_stop_words && self.is_stop_word(&preprocessed) {
                    return Some((None, term_inflections));
                }

                Some((Some(preprocessed.into_owned()), term_inflections))
            })
            .collect();

        SearchTokenizeResult {
            terms,
            should_expand,
        }
    }

    #[inline(never)]
    fn is_stop_word(&self, term: &str) -> bool {
        self.stop_words.iter().any(|t| t == term)
    }

    fn use_default_fault_tolerance(&self) -> bool {
        true
    }

    fn get_best_corrected_term(
        &self,
        _term: &str,
        _dictionary: &BTreeMap<SmartString, &'static TermInfo>,
    ) -> Option<String> {
        None
    }

    fn get_prefix_terms(
        &self,
        _number_of_expanded_terms: usize,
        _term: &str,
        _dictionary: &BTreeMap<SmartString, &'static TermInfo>,
    ) -> Vec<(String, f32)> {
        Vec::new()
    }
}
