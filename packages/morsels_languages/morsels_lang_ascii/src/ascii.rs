#[cfg(feature = "indexer")]
use std::collections::HashSet;

use morsels_common::dictionary::Dictionary;
use morsels_common::utils::split_incl::SplitIncl;
#[cfg(feature = "indexer")]
use regex::Regex;

#[cfg(feature = "indexer")]
use crate::ascii_folding_filter;
use crate::spelling;
use crate::stop_words::get_stop_words;
use crate::utils;
use morsels_common::MorselsLanguageConfig;
#[cfg(feature = "indexer")]
use morsels_common::tokenize::{IndexerTokenizer, TermIter};
use morsels_common::tokenize::{self, SearchTokenizeResult, SearchTokenizer, SearchTokenizeTerm};

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

#[cfg(feature = "indexer")]
impl IndexerTokenizer for Tokenizer {
    fn tokenize<'a>(&'a self, text: &'a mut str) -> TermIter<'a> {
        text.make_ascii_lowercase();
        let it = SENTENCE_SPLITTER.split(text)
            .flat_map(move |sent_slice| {
                sent_slice.split(utils::split_terms)
                    .filter(|&s| !s.is_empty())
                    .map(|term_slice| utils::term_filter(ascii_folding_filter::to_ascii(term_slice)))
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
    fn search_tokenize(
        &self,
        query_chars: &[char],
        query_chars_offset: usize,
        query_chars_offset_end: usize,
        escape_indices: &[usize],
        dict: &Dictionary,
    ) -> SearchTokenizeResult {
        let mut text: String = unsafe { query_chars.get_unchecked(query_chars_offset..query_chars_offset_end) }.iter().collect();
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let mut terms = Vec::new();
        let split: Vec<_> = SplitIncl::split(
            &text,
            utils::split_terms,
        ).collect();

        for (idx, (char_idx, s)) in split.iter().enumerate() {
            if s.is_empty() {
                continue;
            }

            let suffix_wildcard = (idx + 1 != split.len()) && unsafe { split.get_unchecked(idx + 1) }.1 == "*";
            let prefix_ops = tokenize::get_prefix_ops(
                *char_idx + query_chars_offset, 1, query_chars_offset, query_chars, escape_indices, self,
            );

            let mut term_inflections = Vec::new();

            let preprocessed = utils::ascii_and_nonword_filter(&mut term_inflections, s, utils::term_filter);
            if preprocessed.is_empty() {
                continue;
            }

            let original_term = preprocessed.clone().into_owned();
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
                    prefix_ops,
                });
                continue;
            }

            let term = if dict.get_term_info(&preprocessed).is_none() {
                if suffix_wildcard {
                    None
                } else if let Some(corrected_term) = spelling::get_best_corrected_term(dict, &preprocessed) {
                    term_inflections.push(corrected_term.clone());
                    is_corrected = true;
                    Some(corrected_term)
                } else {
                    None
                }
            } else {
                Some(preprocessed.into_owned())
            };

            terms.push(SearchTokenizeTerm {
                term,
                term_inflections,
                original_term,
                suffix_wildcard,
                is_corrected,
                prefix_ops,
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

    fn is_valid_prefix_op_terminator(&self, c: char) -> bool {
        c.is_ascii_whitespace()
    }
}
