mod edit_distance;

use std::ops::Bound::{Excluded, Unbounded};

use smartstring::alias::String;
#[cfg(feature = "perf")]
use wasm_bindgen::JsCast;

use morsels_common::dictionary;

use crate::utils;

pub type Dictionary = dictionary::Dictionary;

static TERM_EXPANSION_ALPHA: f32 = 0.75;  // ceil(0.75x) in https://www.desmos.com/calculator to visualize
static MAXIMUM_TERM_EXPANSION_WEIGHT: f32 = 0.5;  // **total** weight of expanded terms

struct TermWeightPair {
    term: String,
    doc_freq_diff: u32,
}

pub trait SearchDictionary {
    fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String>;

    fn get_prefix_terms(
        &self,
        number_of_expanded_terms: usize,
        base_term: &str,
    ) -> Vec<(std::string::String, f32)>;
}

impl SearchDictionary for Dictionary {
    fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String> {
        #[cfg(feature = "perf")]
        let window: web_sys::Window = js_sys::global().unchecked_into();
        #[cfg(feature = "perf")]
        let performance = window.performance().unwrap();
        #[cfg(feature = "perf")]
        let start = performance.now();

        let mut best_term = None;
        let mut max_doc_freq = 0;

        let base_term_char_count = misspelled_term.chars().count();
        let mut min_edit_distance: usize = match base_term_char_count {
            0..=3 => 1,
            4..=7 => 2,
            _ => 3,
        };

        let mut cache = [255_usize; 255];

        for (term, term_info) in self.term_infos.iter() {
            if term.chars().count().abs_diff(base_term_char_count) > min_edit_distance {
                continue;
            }

            if min_edit_distance == 1 && term_info.doc_freq < max_doc_freq {
                continue;
            }

            let edit_distance = edit_distance::levenshtein(
                term,
                misspelled_term,
                base_term_char_count,
                &mut cache,
            );
            if edit_distance < min_edit_distance {
                min_edit_distance = edit_distance;
                max_doc_freq = term_info.doc_freq;
                best_term = Some(term);
            } else if edit_distance == min_edit_distance && term_info.doc_freq > max_doc_freq {
                max_doc_freq = term_info.doc_freq;
                best_term = Some(term);
            }
        }

        #[cfg(feature = "perf")]
        web_sys::console::log_1(&format!("Spelling correction took {}", performance.now() - start).into());

        if let Some(best_term) = best_term {
            let normal_string = std::string::String::from(&best_term[..]);
            Some(normal_string)
        } else {
            None
        }
    }

    /// Gets terms for prefix search in the following manner:
    /// 
    /// 1. Does a substring check on the terms greater in the BTree,
    ///    using the leftmost TERM_EXPANSION_ALPHA characters of the prefix
    /// 2. Returns `number_of_expanded_terms` terms which have the closest doc freq to the original prefix
    ///    - if the prefix is not a valid term, u32::Max is filled in (i.e. return the most common terms)
    ///    - the terms are weighted in the query according to how long they are (versus the prefix)
    fn get_prefix_terms(
        &self,
        number_of_expanded_terms: usize,
        prefix: &str,
    ) -> Vec<(std::string::String, f32)> {
        let prefix_char_count = prefix.chars().count();

        let prefix_doc_freq = if let Some(term_info) = self.term_infos.get(&String::from(prefix)) {
            term_info.doc_freq
        } else {
            u32::MAX
        };

        // number_of_expanded_terms terms with the closest document frequencies
        let mut top_n: Vec<TermWeightPair> = Vec::with_capacity(100);

        // string to do the prefix check with
        let min_baseterm_substring: String = prefix.chars().take(
            (TERM_EXPANSION_ALPHA * prefix_char_count as f32).ceil() as usize
        ).collect();

        for (term, term_info) in self.term_infos.range((Excluded(min_baseterm_substring.clone()), Unbounded)) {
            if term.starts_with(min_baseterm_substring.as_str()) {
                let doc_freq_diff = term_info.doc_freq.abs_diff(prefix_doc_freq);

                top_n.push(TermWeightPair { term: term.clone(), doc_freq_diff });
            } else {
                break;
            }
        }

        utils::insertion_sort(&mut top_n, |a, b| a.doc_freq_diff.lt(&b.doc_freq_diff));

        let number_of_expanded_terms_found = top_n.len().min(number_of_expanded_terms);
        let max_score_per_expanded_term = MAXIMUM_TERM_EXPANSION_WEIGHT / (number_of_expanded_terms_found as f32);

        top_n.into_iter()
            .take(number_of_expanded_terms_found)
            .map(|TermWeightPair { term, doc_freq_diff: _ }| {
                let length_proportion = prefix_char_count as f32 / term.chars().count() as f32;
                let weight = length_proportion * max_score_per_expanded_term;
                (std::string::String::from(term.as_str()), weight)
            })
            .collect()
    }
}
