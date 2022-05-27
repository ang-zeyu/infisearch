use morsels_common::utils::idf;

use crate::dictionary::SearchDictionary;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

impl Searcher {
    pub fn preprocess(&self, query_parts: &mut Vec<QueryPart>, is_free_text_query: bool) {
        let max_idf = if is_free_text_query {
            query_parts.iter().fold(0.0, |max_idf: f32, query_part| max_idf.max(
                query_part.terms.as_ref().unwrap().iter().fold(0.0, |max_idf, term| {
                    if let Some(term_info) = self.dictionary.get_term_info(term) {
                        max_idf.max(idf::get_idf(self.doc_info.num_docs as f32, term_info.doc_freq as f32))
                    } else {
                        max_idf
                    }
                })
            ))
        } else {
            0.0
        };

        for query_part in query_parts {
            if let Some(terms) = &mut query_part.terms {
                let mut term_idx = 0;
                while term_idx < terms.len() {
                    let term = terms.get(term_idx).unwrap();

                    /*
                     Stop word removal strategy uses idf impact instead of removing all of them.
                     When multiplied by a 100, it should minimally be larger than the maximum idf.
                     */
                    if is_free_text_query && self.tokenizer.is_stop_word(term) {
                        if let Some(term_info) = self.dictionary.get_term_info(term) {
                            let sw_idf = idf::get_idf(self.doc_info.num_docs as f32, term_info.doc_freq as f32);
                            // Hardcoded 100.0 factor for now.
                            if sw_idf * 100.0 < max_idf {
                                query_part.is_stop_word_removed = true;
                                if query_part.original_terms.is_none() {
                                    query_part.original_terms = Some(terms.clone());
                                }
                                terms.remove(term_idx);
                                continue;
                            }
                        }/*  else {
                            unlikely, but let spelling correction handle this
                        } */
                    }

                    /*
                     Spelling correction
                    */
                    if self.dictionary.get_term_info(term).is_none() {
                        query_part.is_corrected = true;
                        if query_part.original_terms.is_none() {
                            query_part.original_terms = Some(terms.clone());
                        }

                        let best_corrected_term = if self.tokenizer.use_default_fault_tolerance() {
                            self.dictionary.get_best_corrected_term(term)
                        } else {
                            self.tokenizer.get_best_corrected_term(term, &self.dictionary.term_infos)
                        };

                        if let Some(corrected_term) = best_corrected_term {
                            terms[term_idx] = corrected_term;
                        } else {
                            terms.remove(term_idx);
                            continue;
                        }
                    }

                    term_idx += 1;
                }
            } else if let Some(children) = &mut query_part.children {
                self.preprocess(children, is_free_text_query);
            }
        }
    }
}
