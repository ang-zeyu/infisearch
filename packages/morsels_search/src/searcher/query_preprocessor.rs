use crate::dictionary::SearchDictionary;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

use super::query_parser::QueryPartType;

// **Total** weight of expanded terms for auto suffix search
const MAXIMUM_TERM_EXPANSION_WEIGHT: f32 = 0.5;

impl Searcher {
    fn is_term_used(term: &str, query_parts: &Vec<QueryPart>) -> bool {
        for query_part in query_parts {
            if let Some(t) = &query_part.term {
                if t == term {
                    return true;
                }
            } else if let Some(children) = &query_part.children {
                if Self::is_term_used(term, children) {
                    return true;
                }
            }
        }

        return false;
    }

    pub fn expand_term_postings_lists(&self, query_parts: &mut Vec<QueryPart>) {
        if query_parts.is_empty()
            || self.searcher_config.searcher_options.max_suffix_search_terms == 0 {
            return;
        }

        self.expand_wildcard_suffix(query_parts);
        self.expand_last_query_part(query_parts);
    }

    fn expand_last_query_part(&self, query_parts: &mut Vec<QueryPart>) {
        let last_query_part = unsafe { query_parts.last_mut().unwrap_unchecked() };
        if !(
            is_expand_candidate(last_query_part)
            && last_query_part.auto_suffix_wildcard
            && !last_query_part.is_subtracted
            && !last_query_part.is_inverted
        ) {
            return;
        }

        let (expanded_terms, old_query_part) = self.begin_expand(
            last_query_part,
            self.searcher_config.searcher_options.max_auto_suffix_search_terms,
        );

        drop(last_query_part);

        let children = Some(
            Self::get_expanded_query_parts(expanded_terms, old_query_part, false, query_parts),
        );

        unsafe { query_parts.last_mut().unwrap_unchecked() }.children = children;
    }

    fn expand_wildcard_suffix(&self, query_parts: &mut Vec<QueryPart>) {
        for query_part in query_parts {
            if is_expand_candidate(query_part) && query_part.suffix_wildcard {
                let (expanded_terms, old_query_part) = self.begin_expand(
                    query_part,
                    self.searcher_config.searcher_options.max_suffix_search_terms,
                );

                query_part.children = Some(
                    Self::get_expanded_query_parts(expanded_terms, old_query_part, true, &Vec::new()),
                );
            } else if let Some(children) = &mut query_part.children {
                self.expand_wildcard_suffix(children);
            }
        }
    }

    /// Performs wildcard suffix search on a QueryPart
    /// 
    /// The QueryPart is replaced with a QueryPartType::Bracket wrapper.
    /// The expanded terms are also returned.
    /// 
    #[inline(never)]
    fn begin_expand(
        &self,
        query_part: &mut QueryPart,
        max_suffix_search_terms: usize,
    ) -> (Vec<(String, f32)>, QueryPart) {
        let term_to_expand = unsafe { query_part.original_term.as_ref().unwrap_unchecked() };

        let term_to_expand_char_count = term_to_expand.chars().count();
        let expanded_terms = self.dictionary.get_prefix_terms(
            &term_to_expand,
            |s| self.tokenizer.is_stop_word(s),
        );

        let num_expanded_terms = expanded_terms.len().min(max_suffix_search_terms);
        let max_score_per_expanded_term = MAXIMUM_TERM_EXPANSION_WEIGHT / (num_expanded_terms as f32);

        let expanded_terms: Vec<_> = expanded_terms.into_iter()
            .map(|term_weight_pair| {
                let term = term_weight_pair.term.as_str();
                let length_proportion = (term_to_expand_char_count as f32) / (term.chars().count() as f32);
                let weight = length_proportion * max_score_per_expanded_term;
                (term.to_owned(), weight)
            })
            .take(num_expanded_terms)
            .collect();
        
        if !expanded_terms.is_empty() {
            query_part.is_suffixed = true;
            if query_part.is_corrected {
                // Delete the corrected term; Expanded terms would be a better match
                query_part.term = None;
                query_part.terms_searched = None;
                query_part.is_corrected = false;
            }
        }

        let mut old_query_part = std::mem::replace(
            query_part,
            QueryPart::get_base(QueryPartType::Bracket),
        );

        query_part.is_mandatory = old_query_part.is_mandatory;
        query_part.is_subtracted = old_query_part.is_subtracted;
        query_part.is_inverted = old_query_part.is_inverted;
        old_query_part.is_mandatory = false;
        old_query_part.is_subtracted = false;
        old_query_part.is_inverted = false;

        (expanded_terms, old_query_part)
    }

    #[inline(never)]
    fn get_expanded_query_parts(
        expanded_terms: Vec<(String, f32)>,
        old_query_part: QueryPart,
        use_old_weight: bool,
        query_parts: &Vec<QueryPart>,
    ) -> Vec<QueryPart> {
        let old_weight = old_query_part.weight;
        let mut wrapper_part_children = Vec::with_capacity(expanded_terms.len() + 1);
        wrapper_part_children.push(old_query_part);

        for (term, weight) in expanded_terms {
            // For auto suffix search, exclude terms that are used in any other part of the query
            // query_parts is an empty Vec::new() for manual suffix
            if !Self::is_term_used(&term, query_parts) {
                wrapper_part_children.push(QueryPart {
                    is_suffixed: true,
                    term: Some(term.to_owned()),
                    terms_searched: Some(vec![term.to_owned()]),
                    weight: if use_old_weight { old_weight } else { weight },
                    ..QueryPart::get_base(QueryPartType::Term)
                });
            }
        }

        return wrapper_part_children;
    }
}

fn is_expand_candidate(query_part: &QueryPart) -> bool {
    matches!(query_part.part_type, QueryPartType::Term)
        && query_part.original_term.is_some()
        // don't further expand query parts that were already from expansion
        && !query_part.is_suffixed
}
