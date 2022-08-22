use morsels_common::utils::idf;

use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

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
}
