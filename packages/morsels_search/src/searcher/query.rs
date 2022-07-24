use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::rc::Rc;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::postings_list::PlIterator;
use crate::postings_list::PostingsList;
use crate::postings_list::TermDoc;
use crate::searcher::query_parser::{self, QueryPart};
use crate::searcher::Searcher;
use crate::utils;

#[derive(Clone)]
struct DocResult {
    doc_id: u32,
    score: f32,
}

impl Eq for DocResult {}

impl PartialEq for DocResult {
    fn eq(&self, other: &Self) -> bool {
        self.doc_id == other.doc_id
    }
}

impl Ord for DocResult {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.score < other.score {
            Ordering::Less
        } else if self.score > other.score {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for DocResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Position {
    pos: u32,
    pl_it_idx: usize,
    pl_it_field_idx: usize,
    pl_it_field_position_idx: usize,
}

impl Eq for Position {}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> Ordering {
        other.pos.cmp(&self.pos)
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.pos.cmp(&self.pos))
    }
}

#[wasm_bindgen]
pub struct Query {
    searched_terms: Vec<Vec<String>>,
    query_parts: Vec<QueryPart>,
    result_heap: BinaryHeap<DocResult>,
    results_retrieved: u32,
    result_limit: Option<u32>,
}

#[wasm_bindgen]
impl Query {
    pub fn get_next_n(&mut self, n: usize) -> Vec<u32> {
        let mut doc_ids: Vec<u32> = Vec::with_capacity(n);
        while !self.result_heap.is_empty()
            && doc_ids.len() < n
            && (self.result_limit.is_none() || self.results_retrieved < self.result_limit.unwrap())
        {
            doc_ids.push(self.result_heap.pop().unwrap().doc_id);
            self.results_retrieved += 1;
        }

        doc_ids
    }

    pub fn get_query_parts(&self) -> String {
        QueryPart::serialize_parts(&self.query_parts)
    }

    pub fn get_searched_terms(&self) -> String {
        let mut output = "[".to_owned();
        let wrapped: Vec<String> = self.searched_terms.iter().map(|term_group| {
            query_parser::serialize_string_vec(term_group)
        }).collect();
        output.push_str(wrapped.join(",").as_str());
        output.push(']');
        output
    }
}

impl Searcher {
    pub fn create_query(
        &self,
        searched_terms: Vec<Vec<String>>,
        query_parts: Vec<QueryPart>,
        postings_lists: Vec<Rc<PostingsList>>,
        result_limit: Option<u32>,
    ) -> Query {
        let max_results = postings_lists
            .iter()
            .max_by_key(|pl| pl.term_docs.len())
            .map(|pl| pl.term_docs.len())
            .unwrap_or(10);
        let mut result_heap: Vec<DocResult> = Vec::with_capacity(max_results);

        let mut pl_its: Vec<PlIterator> = postings_lists
            .iter()
            .enumerate()
            .map(|(idx, pl)| pl.get_it(idx as u8))
            .filter(|pl_it| pl_it.td.is_some())
            .collect();

        // ------------------------------------------
        // Query term proximity ranking
        const PROXIMITY_BASE_SCALING: f32 = 2.5;
        const PROXIMITY_PER_TERM_SCALING: f32 = 0.5;

        let total_proximity_ranking_terms = postings_lists.iter()
            .filter(|pl| pl.include_in_proximity_ranking)
            .count();
        let min_proximity_ranking_terms = ((total_proximity_ranking_terms as f32 / 2.0).ceil() as usize).max(2);
        let proximity_scaling = PROXIMITY_BASE_SCALING
            + (total_proximity_ranking_terms as f32 * PROXIMITY_PER_TERM_SCALING);

        let mut pl_its_for_proximity_ranking: Vec<*const PlIterator> = Vec::with_capacity(pl_its.len());
        let mut position_heap = BinaryHeap::with_capacity(
            total_proximity_ranking_terms as usize * self.searcher_config.num_scored_fields,
        );
        // ------------------------------------------

        loop {
            utils::insertion_sort(&mut pl_its, |a, b| a.lt(b));

            if let Some(&PlIterator { td: Some(lowest_id_term_doc), .. }) = pl_its.first() {
                let curr_doc_id = lowest_id_term_doc.doc_id;
                let mut result = DocResult { doc_id: curr_doc_id, score: 0.0 };
                let mut positional_scaling_factor = 1.0;
    
                // ------------------------------------------
                // Query term proximity ranking
                if self.searcher_config.searcher_options.use_query_term_proximity {
                    proximity_rank(
                        &pl_its,
                        &mut pl_its_for_proximity_ranking,
                        proximity_scaling,
                        &mut position_heap,
                        curr_doc_id,
                        total_proximity_ranking_terms,
                        min_proximity_ranking_terms,
                        &mut positional_scaling_factor
                    );
                }
                // ------------------------------------------
    
                // ------------------------------------------
                // BM25 calculation
    
                for pl_it in pl_its.iter_mut() {
                    if let Some(td) = pl_it.td {
                        if td.doc_id == curr_doc_id {
                            result.score += if td.score != 0.0 {
                                td.score
                            } else {
                                self.calc_doc_bm25_score(td, curr_doc_id, pl_it.pl)
                            };
        
                            pl_it.next();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
    
                result.score *= positional_scaling_factor;
                result_heap.push(result);
    
                // ------------------------------------------
                
            } else {
                // None values are ordered last, finished iterating
                break;
            }
        }

        Query {
            searched_terms,
            query_parts,
            result_heap: BinaryHeap::from(result_heap),
            results_retrieved: 0,
            result_limit,
        }
    }

    /*
     "Soft" disjunctive maximum
     Fields are split into 2 groups: "major" / "minor", with a hardcoded (for now) weight to each.

     The major group contains the highest scoring field, while the minor ones contain the rest,
     which share the 0.3 proportion of the score.
     This avoids penalizing documents that don't have the search term in all fields overly heavily,
     while encouraging matches in multiple fields to some degree.
    */
    pub fn calc_doc_bm25_score(&self, td: &TermDoc, doc_id: u32, pl: &PostingsList) -> f32 {
        const MAJOR_FIELD_FACTOR: f32 = 0.7;
        const MINOR_FIELD_FACTOR: f32 = 0.3;

        let mut doc_term_score = 0.0;
        let mut highest_field_score = 0.0;

        for (field_id, field) in td.fields.iter().enumerate() {
            if field.field_tf > 0.0 {
                let field_info = self.searcher_config.field_infos.get(field_id).unwrap();
                let field_len_factor = self.doc_info.get_doc_length_factor(doc_id as usize, field_id as usize);

                let field_score = ((field.field_tf * (field_info.k + 1.0))
                    / (field.field_tf
                        + field_info.k * (1.0 - field_info.b + field_info.b * field_len_factor)))
                    * field_info.weight;

                if field_score > highest_field_score {
                    highest_field_score = field_score;
                }
                doc_term_score += field_score;
            }
        }

        let minor_fields_score = (doc_term_score - highest_field_score) / self.num_scored_fields_less_one;
        ((MINOR_FIELD_FACTOR * minor_fields_score) + (MAJOR_FIELD_FACTOR * highest_field_score)) * pl.idf as f32 * pl.weight
    }
}


#[inline]
fn proximity_rank<'a>(
    pl_its: &[PlIterator<'a>],
    pl_its_for_proximity_ranking: &mut Vec<*const PlIterator<'a>>,
    proximity_scaling: f32,
    position_heap: &mut BinaryHeap<Position>,
    curr_doc_id: u32,
    total_proximity_ranking_terms: usize,
    min_proximity_ranking_terms: usize,
    scaling_factor: &mut f32,
) {
    const MAX_WINDOW_LEN: u32 = 200;
    const MISSED_TERMS_PENALTY: usize = 4;  // penalty for gaps in terms
    const PROXIMITY_SATURATION: f32 = 4.0;  // how fast it flattens to 1.0

    pl_its_for_proximity_ranking.extend(
        pl_its
            .iter()
            .filter_map(|pl_it| {
                if let Some(td) = pl_it.td {
                    if pl_it.pl.include_in_proximity_ranking
                        && td.doc_id == curr_doc_id {
                        return Some(pl_it as *const PlIterator);
                    }
                }
                None
            }),
    );

    let num_pl_its_curr_doc = pl_its_for_proximity_ranking.len();

    if num_pl_its_curr_doc >= min_proximity_ranking_terms {
        utils::insertion_sort(pl_its_for_proximity_ranking, |&a, &b| unsafe {
            (*a).original_idx.lt(&(*b).original_idx)
        });

        debug_assert!(position_heap.is_empty());

        for (i, &pl_it) in pl_its_for_proximity_ranking.iter().enumerate() {
            let curr_fields = unsafe {
                &(*pl_it).td.as_ref().unwrap().fields
            };
            for (j, curr_field) in curr_fields.iter().enumerate() {
                if let Some(&pos) = curr_field.field_positions.first() {
                    position_heap.push(Position {
                        pos,
                        pl_it_idx: i,
                        pl_it_field_idx: j,
                        pl_it_field_position_idx: 0,
                    });
                }
            }
        }

        let mut next_expected = std::usize::MAX;
        let mut min_window_len = std::u32::MAX;
        let mut min_pos = std::u32::MAX;
        let mut min_terms_missed = min_proximity_ranking_terms
            - (total_proximity_ranking_terms - num_pl_its_curr_doc);
        let mut terms_missed = 0;
        while let Some(mut top) = position_heap.pop() {
            if top.pl_it_idx < next_expected {
                // (Re)start the match from this pl_it
                min_pos = top.pos;
                terms_missed = top.pl_it_idx;
                next_expected = top.pl_it_idx + 1;
            } else if next_expected <= top.pl_it_idx {
                // Continue the match
                terms_missed += top.pl_it_idx - next_expected;
                next_expected = top.pl_it_idx + 1;

                let curr_window_len = top.pos - min_pos;
                let terms_missed = terms_missed + (total_proximity_ranking_terms - next_expected);
                if terms_missed < min_terms_missed {
                    min_terms_missed = terms_missed;
                    min_window_len = curr_window_len;
                } else if terms_missed == min_terms_missed && curr_window_len < min_window_len {
                    min_window_len = curr_window_len;
                    // #[cfg(feature="perf")]
                    // web_sys::console::log_1(&format!("min window len {} {} {}", min_window_len, pos, min_pos).into());
                }
            } else {
                // Restart the match
                next_expected = std::usize::MAX;
            }

            // Update Position iterator
            let doc_field = unsafe {
                &(*pl_its_for_proximity_ranking[top.pl_it_idx]).td.as_ref().unwrap().fields[top.pl_it_field_idx]
            };

            top.pl_it_field_position_idx += 1;
            if let Some(&pos) = doc_field.field_positions.get(top.pl_it_field_position_idx) {
                top.pos = pos;
                position_heap.push(top);
            }
        }

        if min_window_len < MAX_WINDOW_LEN {
            // TODO make this non-linear? (caps off at certain degree)
            min_window_len *= 1 + (min_terms_missed * MISSED_TERMS_PENALTY) as u32;

            if min_window_len < MAX_WINDOW_LEN {
                *scaling_factor = 1.0 + (
                    proximity_scaling
                    /
                    (
                        PROXIMITY_SATURATION
                        + min_window_len as f32
                    )
                );

                /* #[cfg(feature="perf")]
                web_sys::console::log_1(
                    &format!("min_window_len {} terms_in_doc {} min_terms_missed {} scaling_factor {}",
                    min_window_len, num_pl_its_curr_doc, min_terms_missed, scaling_factor,
                ).into()); */
            }
        }
    }

    pl_its_for_proximity_ranking.clear();
}
