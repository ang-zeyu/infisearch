use std::rc::Rc;
use crate::PostingsList::PostingsList;
use futures::future::join_all;
use rustc_hash::FxHashMap;

use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;

use crate::Searcher::Searcher;
use crate::Searcher::query_parser::QueryPart;


impl Searcher {
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

        let mut postings_lists: Vec<&mut PostingsList> = postings_lists_map.values_mut().collect();

        /* let urls = format!("[\"{}/dictionaryTable\",\"{}/dictionaryString\"]", url, url);
        let ptrs: Vec<u32> = vec![0, 0];
        web_sys::console::log_1(&format!("urls {} {}", urls, ptrs.as_ptr() as u32).into());
        
        fetchMultipleArrayBuffers(urls, ptrs.as_ptr() as u32).await?;

        web_sys::console::log_1(&format!("ptrs {} {} took {}", ptrs[0], ptrs[1], performance.now() - start).into()); */

        let window: web_sys::Window = js_sys::global().unchecked_into();
        join_all(
            postings_lists.iter_mut().map(|pl| (*pl).fetch_term(&self.base_url, &window, self.num_scored_fields))
        ).await;

        Ok(postings_lists_map)
    }
}
