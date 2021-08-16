use crate::dictionary::SearchDictionary;
use crate::searcher::Searcher;
use crate::searcher::query_parser::QueryPart;


impl Searcher {
    pub fn preprocess(
        &self,
        query_parts: &mut Vec<QueryPart>,
        is_free_text_query: bool
    ) {
        let allow_stop_word_removal = is_free_text_query && query_parts.len() > 2;
        for query_part in query_parts {
            if let Some(terms) = &mut query_part.terms {
                let mut term_idx = 0;
                while term_idx < terms.len() {
                    let term = terms.get(term_idx).unwrap();
                    if allow_stop_word_removal && self.tokenizer.is_stop_word(term) {
                        query_part.is_stop_word_removed = true;
                        if let None = query_part.original_terms {
                            query_part.original_terms = Option::from(terms.clone());
                        }
                        terms.remove(term_idx);
                        continue;
                    }

                    if let Option::None = self.dictionary.get_term_info(term) {
                        query_part.is_corrected = true;
                        if let None = query_part.original_terms {
                            query_part.original_terms = Option::from(terms.clone());
                        }

                        let best_corrected_term = if self.tokenizer.use_default_trigram() {
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
