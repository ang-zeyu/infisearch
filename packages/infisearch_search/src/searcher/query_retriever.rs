use std::rc::Rc;

use infisearch_common::utils::idf::get_idf;
use infisearch_common::utils::push;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::postings_list::{self, PostingsList};
use crate::searcher::futures::join_all::join_all;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

impl Searcher {
    // Guarantees: postings_lists will have term, term_info populated
    fn populate_term_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        postings_lists: &mut Vec<PostingsList>,
    ) {
        for query_part in query_parts {
            if let Some(term) = &query_part.term {
                if postings_list::get_postings_list(term, postings_lists).is_some() {
                    continue;
                }

                let mut idf = 0.0;
                let term_info = if let Some(term_info) = self.dictionary.get_term_info(term) {
                    idf = get_idf(self.doc_info.num_docs as f32, term_info.doc_freq as f32);
                    Some(term_info.to_owned())
                } else {
                    None
                };

                postings_lists.push(PostingsList {
                    term_docs: Vec::new(),
                    idf,
                    term: Some(term.clone()),
                    term_info,
                });
            } else if let Some(children) = &mut query_part.children {
                self.populate_term_postings_lists(children, postings_lists);
            }
        }
    }

    pub async fn retrieve_term_pls(
        &mut self,
        query_parts: &mut Vec<QueryPart>,
    ) -> Vec<Rc<PostingsList>> {
        let mut postings_lists: Vec<PostingsList> = Vec::new();

        self.populate_term_postings_lists(query_parts, &mut postings_lists);

        let mut pl_numbers: Vec<u32> = postings_lists
            .iter()
            .filter_map(|pl| {
                if let Some(term_info) = &pl.term_info {
                    Some(term_info.postings_file_name)
                } else {
                    None
                }
            })
            .collect();

        // --------------------------------------------
        // Dedup

        if pl_numbers.len() > 1 {
            for i in (1..pl_numbers.len()).rev() {
                if unsafe { pl_numbers.get_unchecked(..i).contains(pl_numbers.get_unchecked(i)) } {
                    pl_numbers.remove(i);
                }
            }
        }

        // --------------------------------------------
        
        let parsed_postings_lists = join_all(
            pl_numbers.into_iter()
                .map(|pl_num| {
                    // Postings lists that are populated from the same pl_num (postings list file)
                    let mut curr_pl_num_pls = Vec::with_capacity(postings_lists.len());

                    for i in (0..postings_lists.len()).rev() {
                        if let Some(term_info) = &unsafe { postings_lists.get_unchecked(i) }.term_info {
                            if pl_num == term_info.postings_file_name {
                                push::push_wo_grow(&mut curr_pl_num_pls, postings_lists.remove(i));
                            }
                        }
                    }

                    #[cfg(feature = "perf")]
                    web_sys::console::log_1(
                        &format!("Retrieving pl {}. Number of terms using it: {}", pl_num, curr_pl_num_pls.len()).into()
                    );

                    self.fetch_pl_into_vec(
                        pl_num,
                        curr_pl_num_pls,
                    )
                })
        ).await;

        for result in parsed_postings_lists {
            if let Some(raw_pl_to_cache) = result.raw_pl_to_cache {
                self.postings_list_cache.add(result.pl_num, raw_pl_to_cache);
            }

            for pl in result.postings_lists {
                push::push_wo_grow(&mut postings_lists, pl);
            }
        }

        postings_lists.into_iter().map(Rc::new).collect()
    }

    /// Fetches a raw postings list file for all PostingList structs that rely on it.
    /// 
    /// Then populates them in `parse_pl`.
    async fn fetch_pl_into_vec(
        &self,
        pl_num: u32,
        mut postings_lists: Vec<PostingsList>
    ) -> FetchPlResult {
        let mut raw_pl_to_cache = None;

        let pl_vec = if let Some(cached) = self.postings_list_cache.get(pl_num) {
            cached
        } else {
            let pl_array_buffer = fetchPl(
                pl_num,
                self.searcher_config.indexing_config.num_pls_per_dir,
                &self.searcher_config.searcher_options.url,
                self.searcher_config.searcher_options.pl_lazy_cache_threshold,
            ).await;
            raw_pl_to_cache = Some(js_sys::Uint8Array::new(&pl_array_buffer).to_vec());

            unsafe { raw_pl_to_cache.as_ref().unwrap_unchecked() }
        } ;
    
        for pl in postings_lists.iter_mut() {
            // Guarantee: pl.term_info is populated at this point
            pl.parse_pl(
                pl_vec,
                &self.invalidation_vector,
                self.searcher_config.num_scored_fields,
                self.searcher_config.indexing_config.with_positions,
            );
        }
    
        FetchPlResult {
            postings_lists,
            pl_num,
            raw_pl_to_cache,
        }
    }
}

struct FetchPlResult {
    postings_lists: Vec<PostingsList>,
    pl_num: u32,
    raw_pl_to_cache: Option<Vec<u8>>,
}

#[wasm_bindgen(module = "/src/searcher/fetchPl.js")]
extern "C" {
    async fn fetchPl(
        pl_name: u32,
        num_pls_per_dir: u32,
        base_url: &str,
        pl_lazy_cache_threshold: u32,
    ) -> JsValue;
}
