pub mod english;

use std::rc::Rc;
use rustc_hash::FxHashMap;
use serde::{Serialize};

pub struct TermInfo {
    pub doc_freq: u32,
    pub idf: f64,
    pub max_term_score: f32,
    pub postings_file_name: u32,
    pub postings_file_offset: u16,
}

pub trait Tokenizer {
    fn tokenize(&self, text: String) -> Vec<String>;

    fn wasm_tokenize(&self, text: String) -> SearchTokenizeResult;

    fn is_stop_word(&self, term: &str) -> bool;

    // If true, simply return Option::None / An empty hashmap for the below two methods
    fn use_default_trigram(&self) -> bool;

    fn get_best_corrected_term(&self, term: &String, dictionary: &FxHashMap<Rc<String>, Rc<TermInfo>>) -> Option<String>;

    fn get_expanded_terms(&self, term: &String, dictionary: &FxHashMap<Rc<String>, Rc<TermInfo>>) -> FxHashMap<String, f32>;
}

#[derive(Serialize)]
pub struct SearchTokenizeResult {
    pub terms: Vec<String>,
    pub should_expand: bool,
}
