use std::borrow::Cow;
#[cfg(feature = "indexer")]
use std::collections::HashSet;

#[cfg(feature = "indexer")]
use regex::Regex;

#[cfg(feature = "indexer")]
use infisearch_common::tokenize::{IndexerTokenizer, TermIter};
use infisearch_common::tokenize::{self, SearchTokenizeResult, SearchTokenizer, SearchTokenizeTerm};
use infisearch_common::InfiLanguageConfig;
use infisearch_common::dictionary::Dictionary;
use infisearch_common::utils::split_incl::SplitIncl;
use infisearch_lang_ascii::{ascii_folding_filter, spelling};
use infisearch_lang_ascii::stop_words::get_stop_words;

use crate::{utils, ts};


#[cfg(feature = "indexer")]
lazy_static! {
    pub static ref SENTENCE_SPLITTER: Regex = Regex::new(
        r#"([.,;?!]\s+)|[\uff0c\u3002\uff01\uff1f\uff1a\uff08\uff09\u201c\u201d]"#,
    ).unwrap();
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

pub fn new_with_options(lang_config: &InfiLanguageConfig) -> Tokenizer {
    let stop_words = get_stop_words(lang_config, &[
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
                SplitIncl::split(sent_slice, |c| utils::split_terms(c) || utils::is_chinese_char(c))
                    .map(|(_, term_slice)| ts::normalize(
                        utils::term_filter(ascii_folding_filter::to_ascii(term_slice)), None,
                    ))
                    .filter(move |term| {
                        let term_byte_len = term.len();
                        term_byte_len > 0
                            && term_byte_len <= self.max_term_len
                            && !(self.ignore_stop_words && self.stop_words.contains(term.as_ref()))
                    })
                    .map(Some)
                    .chain(std::iter::once(None))
            });

        Box::new(it)
    }
}


fn ascii_and_nonword_filter<'a>(term_inflections: &mut Vec<String>, term_slice: &'a str) -> Cow<'a, str> {
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

    let term_filtered = utils::term_filter(ascii_replaced);
    if let Cow::Owned(inner) = term_filtered {
        if !inner.is_empty() {
            term_inflections.push(inner.clone());
        }
        Cow::Owned(inner)
    } else {
        term_filtered
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

        let mut terms: Vec<SearchTokenizeTerm> = Vec::new();
        let split: Vec<_> = SplitIncl::split(
            &text,
            |c| utils::split_terms(c) || utils::is_chinese_char(c),
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

            let preprocessed = ascii_and_nonword_filter(&mut term_inflections, s).to_owned();
            if preprocessed.is_empty() {
                continue;
            }

            let preprocessed = ts::normalize(preprocessed, Some(&mut term_inflections));

            let original_term = preprocessed.clone().into_owned();
            let mut is_corrected = false;

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
                if suffix_wildcard || preprocessed.chars().any(utils::is_chinese_char) {
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
        c.is_ascii_whitespace() || utils::is_chinese_char(c)
    }
}


#[cfg(test)]
mod test {
    use infisearch_common::{InfiLanguageConfig, InfiLanguageConfigOpts};

    use super::Tokenizer;
    use super::IndexerTokenizer;

    fn new() -> Tokenizer {
        let lang_config = InfiLanguageConfig {
            lang: "chinese".to_owned(),
            options: InfiLanguageConfigOpts::default(),
        };

        let stop_words = infisearch_lang_ascii::stop_words::get_stop_words(&lang_config, &[]);
    
        let max_term_len = lang_config.options.max_term_len.unwrap_or(80).min(250);
    
        Tokenizer {
            stop_words,
            ignore_stop_words: lang_config.options.ignore_stop_words.unwrap_or(false),
            max_term_len,
        }
    }

    fn test(s: &str, v: Vec<&str>) {
        let mut s = s.to_owned();
        let tok = new();
        let result: Vec<_> = tok
            .tokenize(&mut s)
            .into_iter()
            .filter_map(|s| s.map(|s| s.into_owned()))
            .collect();
        assert_eq!(result, v);
    }

    #[test]
    fn test_tok() {
        test("AB random day", vec!["ab", "random", "day"]);
        test("AB random我们 day", vec!["ab", "random", "我", "们", "day"]);
        test("AB 我random我 day", vec!["ab", "我", "random", "我", "day"]);
        test("AB我 我sup我reme我 day", vec!["ab", "我", "我", "sup", "我", "reme", "我", "day"]);
        test("AB我 我sup我reme我 day们", vec!["ab", "我", "我", "sup", "我", "reme", "我", "day", "们"]);
    }
}
