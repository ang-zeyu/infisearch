use std::borrow::Cow;
#[cfg(feature = "indexer")]
use std::collections::HashSet;
use std::collections::BTreeMap;

use rust_stemmers::{Algorithm, Stemmer};
use smartstring::alias::String as SmartString;

use morsels_common::MorselsLanguageConfig;
#[cfg(feature = "indexer")]
use morsels_common::tokenize::{IndexerTokenizer, TermIter};
use morsels_common::tokenize::{TermInfo, SearchTokenizeResult, SearchTokenizer};
#[cfg(feature = "indexer")]
use morsels_lang_ascii::ascii_folding_filter;
use morsels_lang_ascii::{ascii::ascii_and_nonword_filter, utils::split_terms};
#[cfg(feature = "indexer")]
use morsels_lang_ascii::ascii::SENTENCE_SPLITTER;
use morsels_lang_ascii::stop_words::get_stop_words;
#[cfg(feature = "indexer")]
use morsels_lang_ascii::utils::term_filter;

pub struct Tokenizer {
    // Remove HashSet from the search binary, where speed benefits are minimal
    #[cfg(feature = "indexer")]
    pub stop_words: HashSet<String>,
    #[cfg(not(feature = "indexer"))]
    pub stop_words: Vec<String>,

    #[cfg(feature = "indexer")]
    ignore_stop_words: bool,

    stemmer: Stemmer,

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

    let stemmer = if let Some(stemmer) = &lang_config.options.stemmer {
        match stemmer.to_lowercase().as_str() {
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

    #[cfg(feature = "indexer")]
    let max_term_len = lang_config.options.max_term_len.unwrap_or(80).min(250);

    Tokenizer {
        stop_words,
        #[cfg(feature = "indexer")]
        ignore_stop_words: lang_config.options.ignore_stop_words.unwrap_or(false),
        stemmer,
        #[cfg(feature = "indexer")]
        max_term_len,
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
                    .filter(move |term_slice| !(self.ignore_stop_words && self.stop_words.contains(term_slice.as_ref())))
                    .map(move |term_slice| {
                        if let Cow::Owned(v) = self.stemmer.stem(&term_slice) {
                            Cow::Owned(v)
                        } else {
                            term_slice
                        }
                    })
                    .filter(move |term| {
                        let term_byte_len = term.len();
                        term_byte_len > 0 && term_byte_len <= self.max_term_len
                    })
                    .map(Some).chain(std::iter::once(None))
            });

        Box::new(it)
    }
}

impl SearchTokenizer for Tokenizer {
    fn search_tokenize(&self, mut text: String, terms_searched: &mut Vec<Vec<String>>) -> SearchTokenizeResult {
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let terms = text
            .split(split_terms)
            .filter(|&s| !s.is_empty())
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
            .filter(|term| term.len() > 0)
            .map(|cow| cow.into_owned())
            .collect();

        SearchTokenizeResult {
            terms,
            should_expand,
        }
    }

    fn is_stop_word(&self, term: &str) -> bool {
        self.stop_words.iter().any(|t| t == term)
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
    ) -> Vec<(String, f32)> {
        Vec::new()
    }
}
