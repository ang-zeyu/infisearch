#[cfg(feature = "indexer")]
use std::borrow::Cow;
#[cfg(feature = "indexer")]
use std::collections::HashSet;

use jieba_rs::Jieba;

#[cfg(feature = "indexer")]
use morsels_common::tokenize::{IndexerTokenizer, TermIter};
use morsels_common::{tokenize::{SearchTokenizeResult, SearchTokenizer, SearchTokenizeTerm}, MorselsLanguageConfig, dictionary::Dictionary};
#[cfg(feature = "indexer")]
use morsels_lang_ascii::ascii_folding_filter;
use morsels_lang_ascii::ascii::ascii_and_nonword_filter;
use morsels_lang_ascii::stop_words::get_stop_words;
#[cfg(feature = "indexer")]
use morsels_lang_ascii::utils::{intra_filter, separating_filter};

#[cfg(feature = "indexer")]
fn term_filter(input: Cow<str>) -> Cow<str> {
    let mut char_iter = input.char_indices()
        .filter(|(_idx, c)| intra_filter(*c) || separating_filter(*c));

    if let Some((char_start, c)) = char_iter.next() {
        let mut output: Vec<u8> = Vec::with_capacity(input.len());
        output.extend_from_slice(input[0..char_start].as_bytes());
        let mut prev_char_end = char_start + c.len_utf8();

        for (char_start, c) in char_iter {
            output.extend_from_slice(input[prev_char_end..char_start].as_bytes());
            prev_char_end = char_start + c.len_utf8();
        }
        output.extend_from_slice(input[prev_char_end..].as_bytes());

        Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
    } else {
        input
    }
}

pub struct Tokenizer {
    // Remove HashSet from the search binary, where speed benefits are minimal
    #[cfg(feature = "indexer")]
    pub stop_words: HashSet<String>,
    #[cfg(not(feature = "indexer"))]
    pub stop_words: Vec<String>,

    ignore_stop_words: bool,

    jieba: Jieba,

    // Just needs to be filtered during indexing
    #[cfg(feature = "indexer")]
    max_term_len: usize,
}

pub fn new_with_options(lang_config: &MorselsLanguageConfig) -> Tokenizer {
    let stop_words = get_stop_words(lang_config, &[
        // TODO
    ]);

    #[cfg(feature = "indexer")]
    let max_term_len = lang_config.options.max_term_len.unwrap_or(80).min(250);

    Tokenizer {
        stop_words,
        ignore_stop_words: lang_config.options.ignore_stop_words.unwrap_or(false),
        jieba: Jieba::empty(),
        #[cfg(feature = "indexer")]
        max_term_len,
    }
}

#[cfg(feature = "indexer")]
impl IndexerTokenizer for Tokenizer {
    fn tokenize<'a>(&'a self, text: &'a mut str) -> TermIter<'a> {
        text.make_ascii_lowercase();
        let it = self.jieba
            .cut(text, false)
            .into_iter()
            .filter(|cut| !cut.trim().is_empty())
            .map(|term| term_filter(ascii_folding_filter::to_ascii(term)))
            .filter_map(move |next| {
                if next.trim().is_empty() {
                    // Punctuation, split on it (None as a sentence separator)
                    Some(None)
                } else if (self.ignore_stop_words && self.stop_words.contains(next.as_ref()))
                    || next.len() > self.max_term_len {
                    // Remove completely
                    None
                } else {
                    Some(Some(next))
                }
            });

        Box::new(it)
    }
}

impl SearchTokenizer for Tokenizer {
    fn search_tokenize(&self, mut text: String, dict: &Dictionary) -> SearchTokenizeResult {
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let terms = self
            .jieba
            .cut_for_search(&text, false)
            .into_iter()
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    return None;
                }

                let mut term_inflections = Vec::new();

                let filtered = ascii_and_nonword_filter(&mut term_inflections, s).trim().to_owned();

                if filtered.is_empty() {
                    return None;
                }

                let original_term = filtered.to_owned();

                if self.ignore_stop_words && self.is_stop_word(&filtered)
                    || dict.get_term_info(&filtered).is_none() {
                    return Some(SearchTokenizeTerm {
                        term: None,
                        term_inflections,
                        original_term,
                    });
                }

                Some(SearchTokenizeTerm {
                    term: Some(filtered),
                    term_inflections,
                    original_term,
                })
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
}
