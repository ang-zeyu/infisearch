use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::rc::Rc;

use rustc_hash::FxHashMap;
use smartstring::alias::String;
use strsim::levenshtein;

use morsels_common::dictionary;

pub type Dictionary = dictionary::Dictionary;

static TERM_EXPANSION_ALPHA: f32 = 0.75;  // ceil(0.75x) in https://www.desmos.com/calculator to visualize
static MAXIMUM_TERM_EXPANSION_WEIGHT: f32 = 0.5;  // **total** weight of expanded terms
static SPELLING_CORRECTION_BASE_ALPHA: f32 = 0.3;

struct TermWeightPair {
    term: Rc<String>,
    idf_difference: f64,
}

impl Eq for TermWeightPair {}

impl PartialEq for TermWeightPair {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
    }
}

impl Ord for TermWeightPair {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.idf_difference > other.idf_difference {
            Ordering::Greater
        } else if self.idf_difference < other.idf_difference {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for TermWeightPair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.idf_difference > other.idf_difference {
            Some(Ordering::Greater)
        } else if self.idf_difference < other.idf_difference {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

pub trait SearchDictionary {
    fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String>;

    fn get_corrected_terms(&self, misspelled_term: &str) -> Vec<Rc<String>>;

    fn get_prefix_terms(
        &self,
        number_of_expanded_terms: usize,
        base_term: &str,
    ) -> FxHashMap<std::string::String, f32>;

    fn get_term_candidates(&self, base_term: &str) -> FxHashMap<Rc<String>, usize>;
}

impl SearchDictionary for Dictionary {
    fn get_best_corrected_term(&self, misspelled_term: &str) -> Option<std::string::String> {
        let mut best_term = None;
        let mut min_idf = f64::MAX;
        for term in self.get_corrected_terms(misspelled_term) {
            let term_info = self.term_infos.get(&term).unwrap();
            if term_info.idf < min_idf {
                min_idf = term_info.idf;
                best_term = Some(term);
            }
        }

        if let Some(best_term) = best_term {
            let normal_string: std::string::String = std::string::String::from(&best_term[..]);
            Some(normal_string)
        } else {
            None
        }
    }

    #[allow(clippy::comparison_chain)]
    fn get_corrected_terms(&self, misspelled_term: &str) -> Vec<Rc<String>> {
        let levenshtein_candidates = self.get_term_candidates(misspelled_term);
        let base_term_char_count = misspelled_term.chars().count();
        let mut min_edit_distance_terms = Vec::new();
        let mut min_edit_distance = 3;

        for (term, score) in levenshtein_candidates {
            // (A intersect B) / (A union B)
            // For n-gram string, there are n - 2 tri-grams
            // Filter edit distance candidates by jacard coefficient first
            if ((score as f32) / ((term.chars().count() + base_term_char_count - score) as f32))
                < SPELLING_CORRECTION_BASE_ALPHA
            {
                continue;
            }

            let edit_distance = levenshtein(&term, misspelled_term);
            if edit_distance >= 3 {
                continue;
            }

            if edit_distance < min_edit_distance {
                min_edit_distance_terms.clear();
                min_edit_distance_terms.push(term);
                min_edit_distance = edit_distance;
            } else if edit_distance == min_edit_distance {
                min_edit_distance_terms.push(term);
            }
        }

        min_edit_distance_terms
    }

    /// Gets terms for prefix search in the following manner:
    /// 
    /// 1. Retrieves candidates using the trigram map
    /// 2. Filter candidates that don't at least have TERM_EXPANSION_ALPHA of the trigrams of the prefix
    /// 3. Does a substring check on the remaining candidates, using the leftmost TERM_EXPANSION_ALPHA characters of the prefix
    /// 4. Returns `number_of_expanded_terms` terms which have the closest idf to the original prefix
    ///    - if the prefix is not a valid term, 0.0 is filled in (i.e. return the most common terms)
    ///    - the terms are weighted in the query according to how long they are (versus the prefix)
    fn get_prefix_terms(
        &self,
        number_of_expanded_terms: usize,
        prefix: &str,
    ) -> FxHashMap<std::string::String, f32> {
        let prefix_char_count = prefix.chars().count();

        let prefix_check_candidates = self.get_term_candidates(prefix);

        let min_matching_trigrams = (TERM_EXPANSION_ALPHA * prefix_char_count as f32).ceil() as usize;

        let prefix_idf = if let Some(term_info) = self.term_infos.get(&String::from(prefix)) {
            term_info.idf
        } else {
            0.0
        };

        // number_of_expanded_terms terms with the closest idfs
        let mut top_n_min_heap: BinaryHeap<TermWeightPair> = BinaryHeap::with_capacity(number_of_expanded_terms);

        // string to do the prefix check with
        let min_baseterm_substring: String = prefix.chars().take(
            (TERM_EXPANSION_ALPHA * prefix_char_count as f32).ceil() as usize
        ).collect();

        for (term, score) in prefix_check_candidates {
            // Filter away candidates that quite match in terms of number of trigrams first
            if score < min_matching_trigrams {
                continue;
            }

            if term.starts_with(min_baseterm_substring.as_str()) && term.as_str() != prefix {
                let term_info = self.term_infos.get(&term).unwrap();
                let idf_difference = (term_info.idf - prefix_idf).abs();

                if top_n_min_heap.len() < number_of_expanded_terms {
                    top_n_min_heap.push(TermWeightPair { term, idf_difference });
                } else if idf_difference < top_n_min_heap.peek().unwrap().idf_difference {
                    top_n_min_heap.pop();
                    top_n_min_heap.push(TermWeightPair { term, idf_difference });
                }
            }
        }

        let number_of_expanded_terms_found = top_n_min_heap.len() as f32;
        let max_score_per_expanded_term = MAXIMUM_TERM_EXPANSION_WEIGHT / number_of_expanded_terms_found;
        let mut expanded_terms: FxHashMap<std::string::String, f32> = FxHashMap::default();
        for TermWeightPair { term, idf_difference: _ } in top_n_min_heap {
            let length_proportion = prefix_char_count as f32 / term.chars().count() as f32;
            let weight = length_proportion * max_score_per_expanded_term;
            expanded_terms.insert(term.to_string(), weight); 
        }

        expanded_terms
    }

    fn get_term_candidates(&self, base_term: &str) -> FxHashMap<Rc<String>, usize> {
        let mut candidates: FxHashMap<Rc<String>, usize> = FxHashMap::default();
        for tri_gram in morsels_common::dictionary::trigrams::get_tri_grams(base_term) {
            if let Some(terms) = self.trigrams.get(tri_gram) {
                for term in terms {
                    match candidates.get_mut(&**term) {
                        Some(val) => {
                            *val += 1;
                        }
                        None => {
                            candidates.insert(Rc::clone(term), 1);
                        }
                    }
                }
            }
        }

        candidates
    }
}
