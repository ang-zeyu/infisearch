use morsels_common::utils::idf;

use crate::dictionary::SearchDictionary;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

use super::query_parser::QueryPartType;

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

        let term_to_expand = last_query_part.original_terms.as_ref().unwrap().first().unwrap();
        let field_name = last_query_part.field_name.clone();
        let expanded_terms = self.dictionary.get_prefix_terms(
            self.searcher_config.searcher_options.max_auto_suffix_search_terms,
            &term_to_expand,
        );

        set_suffixed_info(last_query_part, &expanded_terms);

        for (term, weight) in expanded_terms {
            // For auto suffix search, exclude terms that are used in any other part of the query
            if !Self::is_term_used(&term, query_parts) {
                add_expanded_term(query_parts, term, &field_name, weight);
            }
        }
    }

    fn expand_wildcard_suffix(&self, query_parts: &mut Vec<QueryPart>) {
        for query_part in query_parts {
            if is_expand_candidate(query_part) && query_part.suffix_wildcard {
                let term_to_expand = query_part.original_terms.as_ref().unwrap().first().unwrap();
                let field_name = query_part.field_name.clone();
                let expanded_terms = self.dictionary.get_prefix_terms(
                    self.searcher_config.searcher_options.max_suffix_search_terms,
                    &term_to_expand,
                );

                let mut old_query_part = std::mem::replace(
                    query_part,
                    QueryPart::get_base(QueryPartType::Bracket),
                );

                let weight = old_query_part.weight;
                set_suffixed_info(&mut old_query_part, &expanded_terms);

                let mut wrapper_part_children = Vec::with_capacity(expanded_terms.len() + 1);
                wrapper_part_children.push(old_query_part);

                for (term, _weight) in expanded_terms {
                    add_expanded_term(&mut wrapper_part_children, term, &field_name, weight);
                }

                query_part.children = Some(wrapper_part_children);
            } else if let Some(children) = &mut query_part.children {
                self.expand_wildcard_suffix(children);
            }
        }
    }
}


#[inline(never)]
fn set_suffixed_info(last_query_part: &mut QueryPart, expanded_terms: &Vec<(String, f32)>) {
    if !expanded_terms.is_empty() {
        last_query_part.is_suffixed = true;
        if last_query_part.is_corrected {
            // Delete the corrected term; Expanded terms would be a better match
            last_query_part.terms = None;
            last_query_part.is_corrected = false;
        }
    }
}

#[inline(never)]
fn add_expanded_term(
    wrapper_part_children: &mut Vec<QueryPart>,
    term: String,
    field_name: &Option<String>,
    weight: f32,
) {
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
        weight,
        include_in_proximity_ranking: false,
    });
}

fn is_expand_candidate(query_part: &QueryPart) -> bool {
    matches!(query_part.part_type, QueryPartType::Term)
        && query_part.original_terms.is_some()
        // don't further expand query parts that were already from expansion
        && !query_part.is_suffixed
}
