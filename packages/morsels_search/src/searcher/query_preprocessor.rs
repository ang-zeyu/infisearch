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
        let num_desired_expanded_terms = self.searcher_config.searcher_options.number_of_expanded_terms;
        if query_parts.is_empty() || num_desired_expanded_terms == 0 {
            return;
        }

        let last_query_part = query_parts.last_mut().unwrap();
        if !last_query_part.should_expand {
            return;
        }

        if !matches!(last_query_part.part_type, QueryPartType::Term)
            || last_query_part.original_terms.is_none() {
            last_query_part.should_expand = false;
            return;
        }

        let term_to_expand = last_query_part.original_terms.as_ref().unwrap().first().unwrap();
        let expanded_terms = self.dictionary.get_prefix_terms(
            num_desired_expanded_terms,&term_to_expand,
        );
        let field_name = last_query_part.field_name.clone();

        for (term, weight) in expanded_terms {
            if let Some(_term_info) = self.dictionary.get_term_info(&term) {
                if !Self::is_term_used(&term, query_parts) {
                    query_parts.push(QueryPart {
                        is_corrected: false,
                        is_stop_word_removed: false,
                        should_expand: false,
                        is_expanded: true,
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
            }
        }
    }
}
