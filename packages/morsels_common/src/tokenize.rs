use std::borrow::Cow;

use crate::dictionary::Dictionary;


// When None is yielded, it indicates a positional gap
pub type TermIter<'a> = Box<dyn Iterator<Item = Option<Cow<'a, str>>> + 'a>;

pub trait IndexerTokenizer {
    fn tokenize<'a>(&'a self, text: &'a mut str) -> TermIter<'a>;
}

pub trait SearchTokenizer {
    fn search_tokenize(&self, text: String, dict: &Dictionary) -> SearchTokenizeResult;

    fn is_stop_word(&self, term: &str) -> bool;
}

pub struct SearchTokenizeResult {
    pub should_expand: bool,
    pub terms: Vec<SearchTokenizeTerm>,
}

pub struct SearchTokenizeTerm {
    pub term: Option<String>,
    pub term_inflections: Vec<String>,
    pub original_term: String,
}
