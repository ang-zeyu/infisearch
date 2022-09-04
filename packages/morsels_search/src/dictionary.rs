use std::ops::Bound::{Excluded, Unbounded};

use smartstring::alias::String;

use morsels_common::dictionary;

use crate::utils;

pub type Dictionary = dictionary::Dictionary;

pub struct TermWeightPair {
    pub term: String,
    doc_freq_diff: u32,
}

pub trait SearchDictionary {
    fn get_prefix_terms(
        &self,
        prefix: &str,
    ) -> Vec<TermWeightPair>;
}

impl SearchDictionary for Dictionary {
    /// Gets terms for prefix search in the following manner:
    /// 
    /// 1. Does a substring check on the terms greater in the BTree,
    ///    using the leftmost TERM_EXPANSION_ALPHA characters of the prefix
    /// 2. Returns `number_of_expanded_terms` terms which have the closest doc freq to the original prefix
    ///    - if the prefix is not a valid term, u32::Max is filled in (i.e. return the most common terms)
    ///    - the terms are weighted in the query according to how long they are (versus the prefix)
    #[inline(never)]
    fn get_prefix_terms(
        &self,
        prefix: &str,
    ) -> Vec<TermWeightPair> {
        let prefix_doc_freq = if let Some(term_info) = self.term_infos.get(&String::from(prefix)) {
            term_info.doc_freq
        } else {
            u32::MAX
        };

        // number_of_expanded_terms terms with the closest document frequencies
        let mut top_n: Vec<TermWeightPair> = Vec::with_capacity(100);

        // string to do the prefix check with
        let min_baseterm_substring = String::from(prefix);

        for (term, term_info) in self.term_infos.range((Excluded(min_baseterm_substring.clone()), Unbounded)) {
            if term.starts_with(min_baseterm_substring.as_str()) {
                let doc_freq_diff = term_info.doc_freq.abs_diff(prefix_doc_freq);

                top_n.push(TermWeightPair { term: term.clone(), doc_freq_diff });
            } else {
                break;
            }
        }

        utils::insertion_sort(&mut top_n, |a, b| a.doc_freq_diff.lt(&b.doc_freq_diff));

        top_n
    }
}
