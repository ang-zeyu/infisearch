use infisearch_common::dictionary::Dictionary;

mod edit_distance;

const CACHE_SIZE: usize = 8;

pub struct BestTermCorrector {
    cache: [(String, Option<String>); CACHE_SIZE],
    cache_idx: usize,
}

impl BestTermCorrector {
    #[inline(always)]
    pub fn new() -> Self {
        BestTermCorrector {
            cache: [
                ("".to_owned(), None),
                ("".to_owned(), None),
                ("".to_owned(), None),
                ("".to_owned(), None),
                ("".to_owned(), None),
                ("".to_owned(), None),
                ("".to_owned(), None),
                ("".to_owned(), None),
            ],
            cache_idx: 0,
        }
    }

    #[inline(always)]
    pub fn get_best_corrected_term(&mut self, dict: &Dictionary, misspelled_term: &str) -> Option<String> {
        if let Some((_, corrected)) = self.cache.iter().find(|(base_term, _)| base_term == misspelled_term) {
            return corrected.clone();
        }

        let mut best_term = None;
        let mut max_doc_freq = 0;
    
        let base_term_char_count = misspelled_term.chars().count();
        let mut min_edit_distance: usize = match base_term_char_count {
            0..=4 => 1,
            5..=8 => 2,
            _ => 3,
        };
    
        let mut cache = [255_usize; 255];
    
        for (term, term_info) in dict.term_infos.iter() {
            if term.chars().count().abs_diff(base_term_char_count) > min_edit_distance {
                continue;
            }
    
            if min_edit_distance == 1 && term_info.doc_freq < max_doc_freq {
                continue;
            }
    
            let edit_distance = edit_distance::levenshtein(
                term,
                misspelled_term,
                base_term_char_count,
                &mut cache,
            );
            if edit_distance < min_edit_distance {
                min_edit_distance = edit_distance;
                max_doc_freq = term_info.doc_freq;
                best_term = Some(term);
            } else if edit_distance == min_edit_distance && term_info.doc_freq > max_doc_freq {
                max_doc_freq = term_info.doc_freq;
                best_term = Some(term);
            }
        }
    
        let result = if let Some(best_term) = best_term {
            let normal_string = std::string::String::from(best_term.as_str());
            Some(normal_string)
        } else {
            None
        };

        unsafe {
            *self.cache.get_unchecked_mut(self.cache_idx) = (
                misspelled_term.to_owned(), result.clone(),
            );
        }
        self.cache_idx = (self.cache_idx + 1) % CACHE_SIZE;

        result
    }
}
