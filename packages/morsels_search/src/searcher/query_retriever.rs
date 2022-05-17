use futures::future::join_all;
use rustc_hash::FxHashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::dictionary::SearchDictionary;
use crate::postings_list::PostingsList;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::query_parser::QueryPartType;
use crate::searcher::Searcher;

impl Searcher {
    fn expand_term_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        postings_lists_map: &mut FxHashMap<String, PostingsList>,
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
        let expanded_terms = if self.tokenizer.use_default_trigram() {
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
                if postings_lists_map.get(&term).is_some() {
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

                postings_lists_map.insert(
                    term.clone(),
                    PostingsList {
                        weight,
                        include_in_proximity_ranking: false,
                        term_docs: Vec::new(),
                        idf: term_info.idf,
                        term: Some(term),
                        term_info: Some(term_info.to_owned()),
                        max_term_score: 0.0,
                    },
                );
            }
        }

        query_parts.append(&mut new_query_parts);
    }

    fn populate_term_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        postings_lists: &mut FxHashMap<String, PostingsList>,
    ) {
        for query_part in query_parts {
            if let Some(terms) = &query_part.terms {
                for term in terms {
                    if !postings_lists.contains_key(term) {
                        let mut idf = 0.0;
                        let term_info = if let Some(term_info) = self.dictionary.get_term_info(term) {
                            idf = term_info.idf;
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
                            max_term_score: 0.0,
                        };
                        postings_lists.insert(term.to_owned(), postings_list);
                    }
                }
            } else if let Some(children) = &mut query_part.children {
                self.populate_term_postings_lists(children, postings_lists);
            }
        }
    }

    pub async fn populate_term_pls(
        &self,
        query_parts: &mut Vec<QueryPart>,
    ) -> Result<FxHashMap<String, PostingsList>, JsValue> {
        let mut postings_lists_map: FxHashMap<String, PostingsList> = FxHashMap::default();
        self.populate_term_postings_lists(query_parts, &mut postings_lists_map);

        self.expand_term_postings_lists(query_parts, &mut postings_lists_map);

        let window: web_sys::Window = js_sys::global().unchecked_into();
        join_all(postings_lists_map.values_mut().map(|pl| {
            (*pl).fetch_term(
                &self.searcher_config.searcher_options.url,
                &self.pl_file_cache,
                &self.invalidation_vector,
                &window,
                self.searcher_config.num_scored_fields,
                self.searcher_config.indexing_config.num_pls_per_dir,
                self.searcher_config.indexing_config.with_positions,
            )
        }))
        .await;

        Ok(postings_lists_map)
    }
}
