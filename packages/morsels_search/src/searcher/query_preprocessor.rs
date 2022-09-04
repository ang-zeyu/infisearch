use morsels_common::utils::idf;

use crate::dictionary::SearchDictionary;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

use super::query_parser::QueryPartType;

// **Total** weight of expanded terms for auto suffix search
const MAXIMUM_TERM_EXPANSION_WEIGHT: f32 = 0.5;

impl Searcher {
    pub fn remove_free_text_sw(&self, query_parts: &mut Vec<QueryPart>) {
        let max_idf = query_parts.iter().fold(0.0, |max_idf: f32, query_part| max_idf.max(
            if let Some(terms) = &query_part.terms {
                terms.iter().fold(0.0, |max_idf, term| {
                    if let Some(term_info) = self.dictionary.get_term_info(term) {
                        max_idf.max(idf::get_idf(self.doc_info.num_docs as f32, term_info.doc_freq as f32))
                    } else {
                        max_idf
                    }
                })
            } else {
                0.0
            }
        ));

        for query_part in query_parts {
            if let Some(terms) = &mut query_part.terms {
                debug_assert!(terms.len() <= 1);

                if let Some(term) = terms.first() {
                    /*
                     Stop word removal strategy uses idf impact instead of removing all of them.
                     When multiplied by a 100, it should minimally be larger than the maximum idf.
                     */
                    if self.tokenizer.is_stop_word(term) {
                        if let Some(term_info) = self.dictionary.get_term_info(term) {
                            let sw_idf = idf::get_idf(self.doc_info.num_docs as f32, term_info.doc_freq as f32);
                            // Hardcoded 100.0 factor for now.
                            if sw_idf * 100.0 < max_idf {
                                query_part.is_stop_word_removed = true;
                                terms.pop();
                            }
                        }/*  else {
                            spelling correction handles this
                        } */
                    }
                }
            }
        }
    }

    fn is_term_used(term: &str, query_parts: &Vec<QueryPart>) -> bool {
        for query_part in query_parts {
            if let Some(terms) = &query_part.terms {
                if terms.iter().any(|t| t == term) {
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
        let last_query_part = query_parts.last_mut().unwrap();
        if !(is_expand_candidate(last_query_part) && last_query_part.auto_suffix_wildcard) {
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

        query_parts.last_mut().unwrap().children = children;
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
        let term_to_expand = query_part.original_terms.as_ref().unwrap().first().unwrap();

        let term_to_expand_char_count = term_to_expand.chars().count();
        let expanded_terms = self.dictionary.get_prefix_terms(
            &term_to_expand,
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
                query_part.terms = None;
                query_part.is_corrected = false;
            }
        }

        let old_query_part = std::mem::replace(
            query_part,
            QueryPart::get_base(QueryPartType::Bracket),
        );

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
        let field_name = old_query_part.field_name.clone();
        let mut wrapper_part_children = Vec::with_capacity(expanded_terms.len() + 1);
        wrapper_part_children.push(old_query_part);

        for (term, weight) in expanded_terms {
            // For auto suffix search, exclude terms that are used in any other part of the query
            // query_parts is an empty Vec::new() for manual suffix search
            if !Self::is_term_used(&term, query_parts) {
                wrapper_part_children.push(QueryPart {
                    is_corrected: false,
                    is_stop_word_removed: false,
                    auto_suffix_wildcard: false,
                    suffix_wildcard: false,
                    is_suffixed: true,
                    original_terms: None,
                    terms: Some(vec![term.clone()]),
                    terms_searched: Some(vec![vec![term.clone()]]),
                    part_type: QueryPartType::Term,
                    field_name: field_name.clone(),
                    children: None,
                    weight: if use_old_weight { old_weight } else { weight },
                    include_in_proximity_ranking: false,
                });
            }
        }

        return wrapper_part_children;
    }
}

fn is_expand_candidate(query_part: &QueryPart) -> bool {
    matches!(query_part.part_type, QueryPartType::Term)
        && query_part.original_terms.is_some()
        // don't further expand query parts that were already from expansion
        && !query_part.is_suffixed
}
