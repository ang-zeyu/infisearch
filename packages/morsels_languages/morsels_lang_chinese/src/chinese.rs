#[cfg(feature = "indexer")]
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

use jieba_rs::Jieba;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use smartstring::alias::String as SmartString;

#[cfg(feature = "indexer")]
use morsels_common::tokenize::IndexerTokenizer;
use morsels_common::tokenize::{TermInfo, SearchTokenizeResult, SearchTokenizer};
#[cfg(feature = "indexer")]
use morsels_lang_ascii::ascii_folding_filter;
use morsels_lang_ascii::ascii::ascii_and_nonword_filter;
#[cfg(feature = "indexer")]
use morsels_lang_ascii::utils::intra_filter;

#[cfg(feature = "indexer")]
fn term_filter(input: Cow<str>) -> Cow<str> {
    let mut char_iter = input.char_indices().filter(|(_idx, c)| intra_filter(*c));

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

fn get_stop_words_set(stop_words_vec: Vec<String>) -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::default();

    for word in stop_words_vec {
        set.insert(word.to_owned());
    }

    set
}

pub struct Tokenizer {
    pub stop_words: HashSet<String>,
    #[cfg(feature = "indexer")]
    ignore_stop_words: bool,
    jieba: Jieba,
    max_term_len: usize,
}

fn get_default_max_term_len() -> usize {
    80
}

#[derive(Deserialize)]
pub struct TokenizerOptions {
    stop_words: Option<Vec<String>>,
    #[cfg(feature = "indexer")]
    #[serde(default)]
    ignore_stop_words: bool,
    #[serde(default = "get_default_max_term_len")]
    max_term_len: usize,
}

pub fn new_with_options(options: TokenizerOptions) -> Tokenizer {
    let stop_words = if let Some(stop_words) = options.stop_words {
        get_stop_words_set(stop_words)
    } else {
        get_stop_words_set(Vec::new())
    };

    Tokenizer {
        stop_words,
        #[cfg(feature = "indexer")]
        ignore_stop_words: options.ignore_stop_words,
        jieba: Jieba::empty(),
        max_term_len: options.max_term_len.min(250)
    }
}

#[cfg(feature = "indexer")]
impl IndexerTokenizer for Tokenizer {
    fn tokenize<'a>(&self, text: &'a mut str) -> Vec<Vec<Cow<'a, str>>> {
        text.make_ascii_lowercase();
        self.jieba
            .cut(text, false)
            .into_iter()
            .filter(|cut| !cut.trim().is_empty())
            .map(|term| term_filter(ascii_folding_filter::to_ascii(term)))
            .fold(vec![Vec::new()], |mut acc, next| {
                if next.trim().is_empty() {
                    acc.push(Vec::new()); // Split on punctuation
                } else if !(self.ignore_stop_words && self.stop_words.contains(next.as_ref()))
                    && next.len() <= self.max_term_len {
                    acc.last_mut().unwrap().push(next);
                }
                acc
            })
    }
}

impl SearchTokenizer for Tokenizer {
    fn search_tokenize(&self, mut text: String, terms_searched: &mut Vec<Vec<String>>) -> SearchTokenizeResult {
        text.make_ascii_lowercase();

        let should_expand = !text.ends_with(' ');

        let terms = self
            .jieba
            .cut_for_search(&text, false)
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let mut terms = vec![s.to_owned()];

                let filtered = ascii_and_nonword_filter(&mut terms, s).into_owned();

                terms_searched.push(terms);

                filtered
            })
            .filter(|s| !s.trim().is_empty() && s.len() <= self.max_term_len)
            .collect();

        SearchTokenizeResult { terms, should_expand }
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
