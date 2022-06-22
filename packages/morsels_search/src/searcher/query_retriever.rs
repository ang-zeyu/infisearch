use morsels_common::utils::idf::get_idf;

use crate::dictionary::SearchDictionary;
use crate::postings_list::{self, PostingsList};
use crate::searcher::futures::join_all::join_all;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::query_parser::QueryPartType;
use crate::searcher::Searcher;

impl Searcher {
    fn expand_term_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        postings_lists: &mut Vec<(String, PostingsList)>,
    ) {
        let num_desired_expanded_terms = self.searcher_config.searcher_options.number_of_expanded_terms;
        if query_parts.is_empty() || num_desired_expanded_terms == 0 {
            return;
        }

        let last_query_part = query_parts.last_mut().unwrap();
        if !last_query_part.should_expand {
            return;
        }

        if !matches!(last_query_part.part_type, QueryPartType::Term) {
            last_query_part.should_expand = false;
            return;
        }

        if last_query_part.original_terms.is_none() {
            last_query_part.original_terms = last_query_part.terms.clone();
        } /* else {
            from spelling correction / stop word removal
        } */

        let term_to_expand = last_query_part.original_terms.as_ref().unwrap().first().unwrap();
        let expanded_terms = if self.tokenizer.use_default_fault_tolerance() {
            self.dictionary.get_prefix_terms(num_desired_expanded_terms,term_to_expand)
        } else {
            self.tokenizer.get_prefix_terms(
num_desired_expanded_terms,
                term_to_expand,
                &self.dictionary.term_infos,
            )
        };

        if expanded_terms.is_empty() {
            return;
        }

        let mut new_query_parts: Vec<QueryPart> = Vec::with_capacity(expanded_terms.len());
        for (term, weight) in expanded_terms {
            if let Some(term_info) = self.dictionary.get_term_info(&term) {
                if postings_list::get_postings_list(&term, &postings_lists).is_some() {
                    continue;
                }

                new_query_parts.push(QueryPart {
                    is_corrected: false,
                    is_stop_word_removed: false,
                    should_expand: false,
                    is_expanded: true,
                    original_terms: None,
                    terms: Some(vec![term.clone()]),
                    part_type: QueryPartType::Term,
                    field_name: last_query_part.field_name.clone(),
                    children: None,
                });

                postings_lists.push((
                    term.clone(),
                    PostingsList {
                        weight,
                        include_in_proximity_ranking: false,
                        term_docs: Vec::new(),
                        idf: get_idf(self.doc_info.num_docs as f32, term_info.doc_freq as f32),
                        term: Some(term),
                        term_info: Some(term_info.to_owned()),
                    },
                ));
            }
        }

        query_parts.append(&mut new_query_parts);
    }

    fn populate_term_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        postings_lists: &mut Vec<(String, PostingsList)>,
    ) {
        for query_part in query_parts {
            if let Some(terms) = &query_part.terms {
                for term in terms {
                    if postings_list::get_postings_list(term, &postings_lists).is_some() {
                        continue;
                    }

                    let mut idf = 0.0;
                    let term_info = if let Some(term_info) = self.dictionary.get_term_info(term) {
                        idf = get_idf(self.doc_info.num_docs as f32, term_info.doc_freq as f32);
                        Some(term_info.to_owned())
                    } else {
                        None
                    };
                    let postings_list = PostingsList {
                        weight: 1.0,
                        include_in_proximity_ranking: true,
                        term_docs: Vec::new(),
                        idf,
                        term: Some(term.clone()),
                        term_info,
                    };
                    postings_lists.push((term.to_owned(), postings_list));
                }
            } else if let Some(children) = &mut query_part.children {
                self.populate_term_postings_lists(children, postings_lists);
            }
        }
    }

    pub async fn retrieve_term_pls(
        &self,
        query_parts: &mut Vec<QueryPart>,
    ) -> Vec<(String, PostingsList)> {
        let mut postings_lists: Vec<(String, PostingsList)> = Vec::new();
        self.populate_term_postings_lists(query_parts, &mut postings_lists);

        self.expand_term_postings_lists(query_parts, &mut postings_lists);

        join_all(postings_lists.iter_mut().map(|(_, pl)| {
            pl.fetch_term(
                &self.searcher_config.searcher_options.url,
                &self.invalidation_vector,
                self.searcher_config.num_scored_fields,
                self.searcher_config.indexing_config.num_pls_per_dir,
                self.searcher_config.indexing_config.with_positions,
            )
        }))
        .await;

        postings_lists
    }
}
