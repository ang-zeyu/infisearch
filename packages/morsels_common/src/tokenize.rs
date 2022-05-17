use rustc_hash::FxHashMap;
use smartstring::alias::String as SmartString;
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct TermInfo {
    pub doc_freq: u32,
    pub idf: f64,
    pub postings_file_name: u32,
    pub postings_file_offset: u32,
}

#[cfg(test)]
impl Eq for TermInfo {}

#[cfg(test)]
impl PartialEq for TermInfo {
    fn eq(&self, other: &Self) -> bool {
        self.doc_freq == other.doc_freq
            && self.postings_file_name == other.postings_file_name
            && self.postings_file_offset == other.postings_file_offset
    }
}

pub trait IndexerTokenizer {
    fn tokenize<'a>(&self, text: &'a mut str) -> Vec<Vec<Cow<'a, str>>>;
}

pub trait SearchTokenizer {
    fn search_tokenize(&self, text: String, terms_searched: &mut Vec<Vec<String>>) -> SearchTokenizeResult;

    fn is_stop_word(&self, term: &str) -> bool;

    // If true, simply return None / An empty hashmap for the below two methods
    fn use_default_trigram(&self) -> bool;

    fn get_best_corrected_term(
        &self,
        term: &str,
        dictionary: &FxHashMap<Rc<SmartString>, TermInfo>,
    ) -> Option<String>;

    fn get_prefix_terms(
        &self,
        number_of_expanded_terms: usize,
        term: &str,
        dictionary: &FxHashMap<Rc<SmartString>, TermInfo>,
    ) -> FxHashMap<String, f32>;
}

pub struct SearchTokenizeResult {
    pub terms: Vec<String>,
    pub should_expand: bool,
}
