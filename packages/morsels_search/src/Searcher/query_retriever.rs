use std::rc::Rc;

use futures::future::join_all;
use rustc_hash::FxHashMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;

use crate::dictionary::SearchDictionary;
use crate::postings_list::PostingsList;
use crate::searcher::Searcher;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::query_parser::QueryPartType;


impl Searcher {
    fn expand_term_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        postings_lists_map: &mut FxHashMap<String, PostingsList>,
    ) {
        if query_parts.len() == 0 {
            return;
        }

        let last_query_part = query_parts.last_mut().unwrap();
        if self.searcher_config.searcher_options.number_of_expanded_terms > 0
            && matches!(last_query_part.part_type, QueryPartType::TERM)
            && last_query_part.should_expand
            && !last_query_part.is_stop_word_removed {
            if let None = last_query_part.original_terms {
                last_query_part.original_terms = Option::from(last_query_part.terms.clone());
            }

            let expanded_terms = if self.tokenizer.use_default_trigram() {
                self.dictionary.get_expanded_terms(
                    self.searcher_config.searcher_options.number_of_expanded_terms,
                    last_query_part.original_terms.as_ref().unwrap().get(0).unwrap()
                )
            } else {
                self.tokenizer.get_expanded_terms(
                    self.searcher_config.searcher_options.number_of_expanded_terms,
                    last_query_part.original_terms.as_ref().unwrap().get(0).unwrap(),
                    &self.dictionary.term_infos
                )
            };

            last_query_part.is_expanded = expanded_terms.len() > 0;

            let mut new_query_parts: Vec<QueryPart> = Vec::with_capacity(expanded_terms.len());
            for (term, weight) in expanded_terms {
                if let Some(term_info) = self.dictionary.get_term_info(&term) {
                    if let None = postings_lists_map.get(&term) {
                        last_query_part.terms.as_mut().unwrap().push(term.clone());

                        new_query_parts.push(QueryPart {
                            is_corrected: false,
                            is_stop_word_removed: false,
                            should_expand: false,
                            is_expanded: false,
                            original_terms: None,
                            terms: Option::from(vec![term.clone()]),
                            part_type: QueryPartType::TERM,
                            field_name: last_query_part.field_name.clone(),
                            children: None,
                        });

                        postings_lists_map.insert(term.clone(), PostingsList {
                            weight,
                            include_in_proximity_ranking: false,
                            term_docs: Vec::new(),
                            idf: term_info.idf,
                            term: Some(term),
                            term_info: Some(Rc::clone(term_info)),
                            max_term_score: 0.0,
                        });
                    }
                }
            }

            drop(last_query_part);
            query_parts.append(&mut new_query_parts);
        }
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
                        let term_info = if let Some(term_info_rc) = self.dictionary.get_term_info(term) {
                            idf = term_info_rc.idf;
                            Option::from(Rc::clone(term_info_rc))
                        } else {
                            Option::None
                        };
                        let postings_list = PostingsList {
                            weight: 1.0,
                            include_in_proximity_ranking: true,
                            term_docs: Vec::new(),
                            idf,
                            term: Option::from(term.clone()),
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

        let postings_lists: Vec<&mut PostingsList> = postings_lists_map.values_mut().collect();

        /* let urls = format!("[\"{}/dictionaryTable\",\"{}/dictionaryString\"]", url, url);
        let ptrs: Vec<u32> = vec![0, 0];
        web_sys::console::log_1(&format!("urls {} {}", urls, ptrs.as_ptr() as u32).into());
        
        fetchMultipleArrayBuffers(urls, ptrs.as_ptr() as u32).await?;

        web_sys::console::log_1(&format!("ptrs {} {} took {}", ptrs[0], ptrs[1], performance.now() - start).into()); */

        let window: web_sys::Window = js_sys::global().unchecked_into();
        join_all(
            postings_lists.into_iter().map(|pl| (*pl).fetch_term(
                &self.searcher_config.searcher_options.url,
                &self.pl_file_cache,
                &self.invalidation_vector,
                &window,
                self.searcher_config.num_scored_fields,
                self.searcher_config.indexing_config.num_pls_per_dir,
                self.searcher_config.indexing_config.with_positions
            ))
        ).await;

        Ok(postings_lists_map)
    }
}
