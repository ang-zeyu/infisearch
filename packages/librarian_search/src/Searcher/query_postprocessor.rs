use std::rc::Rc;
use futures::future::join_all;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use crate::PostingsList::PostingsList;
use crate::Searcher::Searcher;
use crate::Searcher::query_parser::QueryPartType;
use crate::Searcher::query_parser::QueryPart;

impl Searcher {
    pub async fn postprocess(
        &self,
        query_parts: &mut Vec<QueryPart>,
        postings_lists: &mut Vec<Rc<PostingsList>>,
    ) -> Result<(), JsValue> {
        let query_parts_len = query_parts.len();
        if let Some(last_query_part) = query_parts.get_mut(query_parts_len - 1) {
            if self.searcher_options.use_query_term_expansion
              && matches!(last_query_part.partType, QueryPartType::TERM)
              && last_query_part.shouldExpand
              && !last_query_part.isCorrected {
                if let None = last_query_part.originalTerms {
                    last_query_part.originalTerms = Option::from(last_query_part.terms.clone());
                }

                let expanded_terms = if self.tokenizer.use_default_trigram() {
                    self.dictionary.get_expanded_terms(
                        last_query_part.terms.as_ref().unwrap().get(0).unwrap()
                    )
                } else {
                    self.tokenizer.get_expanded_terms(
                        last_query_part.terms.as_ref().unwrap().get(0).unwrap(),
                        &self.dictionary.term_infos
                    )
                };

                last_query_part.isExpanded = expanded_terms.len() > 0;

                let mut expanded_postings_lists: Vec<PostingsList> = expanded_terms
                    .into_iter()
                    .map(|(term, weight)| {
                        if let Some(term_info) = self.dictionary.get_term_info(&term) {
                            let pl = PostingsList {
                                weight,
                                include_in_proximity_ranking: false,
                                term_docs: Vec::new(),
                                idf: term_info.idf,
                                term: Option::None,
                                term_info: Option::None,
                            };

                            last_query_part.terms.as_mut().unwrap().push(term);

                            Option::from(pl)
                        } else {
                            Option::None
                        }
                    })
                    .filter(|opt| opt.is_some())
                    .map(|opt| opt.unwrap())
                    .collect();

                let window: web_sys::Window = js_sys::global().unchecked_into();
                join_all(
                    expanded_postings_lists.iter_mut().map(|pl| (*pl).fetch_term(&self.base_url, &window, self.num_scored_fields))
                ).await;

                postings_lists.append(&mut expanded_postings_lists.into_iter().map(|pl| Rc::new(pl)).collect());
            }
        }

        Ok(())
    }
}
