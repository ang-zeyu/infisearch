use std::borrow::Cow;
#[cfg(feature = "indexer")]
use std::collections::HashSet;

use rust_stemmers::{Algorithm, Stemmer};

use morsels_common::{MorselsLanguageConfig, dictionary::Dictionary, tokenize::SearchTokenizeTerm, utils::split_incl::SplitIncl};
#[cfg(feature = "indexer")]
use morsels_common::tokenize::{IndexerTokenizer, TermIter};
use morsels_common::tokenize::{SearchTokenizeResult, SearchTokenizer};
#[cfg(feature = "indexer")]
use morsels_lang_ascii::ascii_folding_filter;
use morsels_lang_ascii::{utils as ascii_utils, spelling};
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
                sent_slice.split(ascii_utils::split_terms)
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
    fn search_tokenize(&self, mut text: String, dict: &Dictionary) -> SearchTokenizeResult {
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let mut terms = Vec::new();
        let split: Vec<_> = SplitIncl::split(
            &text,
            ascii_utils::split_terms,
        ).collect();

        for (idx, s) in split.iter().enumerate() {
            if s.is_empty() {
                continue;
            }

            let suffix_wildcard = (idx + 1 != split.len()) && split[idx + 1] == "*";

            let mut term_inflections = Vec::new();

            let preprocessed = ascii_utils::ascii_and_nonword_filter(&mut term_inflections, s, ascii_utils::term_filter);
            let stemmed = if let Cow::Owned(v) = self.stemmer.stem(&preprocessed) {
                term_inflections.push(v.clone());
                Cow::Owned(v)
            } else {
                preprocessed.clone()
            };

            if stemmed.is_empty() {
                continue;
            }

            let original_term = stemmed.clone().into_owned();
            let mut is_corrected = false;

            // This comes before spelling correction,
            // as ignore_stop_words removes from the index (won't be present in the dictionary)
            if self.ignore_stop_words && self.is_stop_word(&preprocessed) {
                terms.push(SearchTokenizeTerm {
                    term: None,
                    term_inflections,
                    original_term,
                    suffix_wildcard,
                    is_corrected,
                });
                continue;
            }

            let term = if dict.get_term_info(&stemmed).is_none() {
                if suffix_wildcard {
                    None
                } else if let Some(corrected_term) = spelling::get_best_corrected_term(dict, &stemmed) {
                    term_inflections.push(corrected_term.clone());
                    is_corrected = true;
                    Some(corrected_term)
                } else {
                    None
                }
            } else {
                Some(stemmed.into_owned())
            };

            terms.push(SearchTokenizeTerm {
                term,
                term_inflections,
                original_term,
                suffix_wildcard,
                is_corrected,
            })
        }

        SearchTokenizeResult {
            terms,
            auto_suffix_wildcard: should_expand,
        }
    }

    #[inline(never)]
    fn is_stop_word(&self, term: &str) -> bool {
        self.stop_words.iter().any(|t| t == term)
    }
}
