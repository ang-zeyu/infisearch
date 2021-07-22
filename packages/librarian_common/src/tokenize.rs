pub mod english;

use rustc_hash::FxHashMap;
use crate::dictionary::Dictionary;
use serde::{Serialize};

pub trait Tokenizer {
    fn tokenize(&self, text: String) -> Vec<String>;

    fn wasm_tokenize(&self, text: String) -> WasmTokenizeResult;

    fn is_stop_word(&self, term: &str) -> bool;

    // If true, simply return Option::None / An empty hashmap for the below two methods
    fn use_default_trigram(&self) -> bool;

    fn get_best_corrected_term(&self, term: &String, dictionary: &Dictionary) -> Option<String>;

    fn get_expanded_terms(&self, term: &String, dictionary: &Dictionary) -> FxHashMap<String, f32>;
}

#[derive(Serialize)]
pub struct WasmTokenizeResult {
    pub terms: Vec<String>,
    pub should_expand: bool,
}
