mod edit_distance;

use std::ops::Bound::{Excluded, Unbounded};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use smartstring::alias::String;
#[cfg(feature = "perf")]
use wasm_bindgen::JsCast;

use morsels_common::dictionary;

pub type Dictionary = dictionary::Dictionary;

static TERM_EXPANSION_ALPHA: f32 = 0.75;  // ceil(0.75x) in https://www.desmos.com/calculator to visualize
static MAXIMUM_TERM_EXPANSION_WEIGHT: f32 = 0.5;  // **total** weight of expanded terms

struct TermWeightPair {
    term: String,
    doc_freq_diff: u32,
}

impl Eq for TermWeightPair {}

impl PartialEq for TermWeightPair {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
    }
}

impl Ord for TermWeightPair {
    fn cmp(&self, other: &Self) -> Ordering {
        self.doc_freq_diff.cmp(&other.doc_freq_diff)
    }
}

impl PartialOrd for TermWeightPair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.doc_freq_diff.partial_cmp(&other.doc_freq_diff)
    }
}

pub trait SearchDictionary {
    fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String>;

    fn get_prefix_terms(
        &self,
        number_of_expanded_terms: usize,
        base_term: &str,
    ) -> HashMap<std::string::String, f32>;
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
    ) -> HashMap<std::string::String, f32> {
        let prefix_char_count = prefix.chars().count();

        let prefix_doc_freq = if let Some(term_info) = self.term_infos.get(&String::from(prefix)) {
            term_info.doc_freq
        } else {
            u32::MAX
        };

        // number_of_expanded_terms terms with the closest document frequencies
        let mut top_n_heap: BinaryHeap<TermWeightPair> = BinaryHeap::with_capacity(number_of_expanded_terms);

        // string to do the prefix check with
        let min_baseterm_substring: String = prefix.chars().take(
            (TERM_EXPANSION_ALPHA * prefix_char_count as f32).ceil() as usize
        ).collect();

        for (term, term_info) in self.term_infos.range((Excluded(min_baseterm_substring.clone()), Unbounded)) {
            if term.starts_with(min_baseterm_substring.as_str()) {
                let doc_freq_diff = term_info.doc_freq.abs_diff(prefix_doc_freq);

                if top_n_heap.len() < number_of_expanded_terms {
                    top_n_heap.push(TermWeightPair { term: term.clone(), doc_freq_diff });
                } else if doc_freq_diff < top_n_heap.peek().unwrap().doc_freq_diff {
                    top_n_heap.pop();
                    top_n_heap.push(TermWeightPair { term: term.clone(), doc_freq_diff });
                }
            } else {
                break;
            }
        }

        let number_of_expanded_terms_found = top_n_heap.len() as f32;
        let max_score_per_expanded_term = MAXIMUM_TERM_EXPANSION_WEIGHT / number_of_expanded_terms_found;
        let mut expanded_terms: HashMap<std::string::String, f32> = HashMap::default();
        for TermWeightPair { term, doc_freq_diff: _ } in top_n_heap {
            let length_proportion = prefix_char_count as f32 / term.chars().count() as f32;
            let weight = length_proportion * max_score_per_expanded_term;
            expanded_terms.insert(term.to_string(), weight); 
        }

        expanded_terms
    }
}
