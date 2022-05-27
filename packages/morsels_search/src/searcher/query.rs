use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::rc::Rc;

use serde::{Serialize, Serializer};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::postings_list::PlIterator;
use crate::postings_list::PostingsList;
use crate::postings_list::TermDoc;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

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

impl Serialize for DocResult {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        (self.doc_id, self.score).serialize(serializer)
    }
}

struct Position {
    pos: u32,
    pl_it_idx: usize,
    pl_it_field_idx: usize,
    pl_it_field_fieldposition_next_idx: usize,
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
    pub fn get_next_n(&mut self, n: usize) -> JsValue {
        let mut doc_ids: Vec<DocResult> = Vec::with_capacity(n);
        while !self.result_heap.is_empty()
            && doc_ids.len() < n
            && (self.result_limit.is_none() || self.results_retrieved < self.result_limit.unwrap())
        {
            doc_ids.push(self.result_heap.pop().unwrap());
            self.results_retrieved += 1;
        }

        JsValue::from_serde(&doc_ids).unwrap()
    }

    pub fn get_query_parts(&self) -> JsValue {
        JsValue::from_serde(&self.query_parts).unwrap()
    }

    pub fn get_searched_terms(&self) -> JsValue {
        JsValue::from_serde(&self.searched_terms).unwrap()
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
        let mut result_heap: BinaryHeap<DocResult> = BinaryHeap::new();

        let mut pl_its: Vec<PlIterator> = postings_lists
            .iter()
            .enumerate()
            .map(|(idx, pl)| pl.get_it(idx as u8))
            .filter(|pl_it| pl_it.td.is_some())
            .collect();
        let mut pl_its_for_proximity_ranking: Vec<*const PlIterator> = Vec::with_capacity(pl_its.len());
        pl_its.sort();

        let total_proximity_ranking_terms =
            postings_lists.iter().filter(|pl| pl.include_in_proximity_ranking).count() as f32;
        let proximity_ranking_max_scale = total_proximity_ranking_terms * 1.8;

        while !pl_its.is_empty() {
            let curr_doc_id = pl_its.first().unwrap().td.unwrap().doc_id;
            let mut result = DocResult { doc_id: curr_doc_id, score: 0.0 };
            let mut scaling_factor = 1.0;

            // ------------------------------------------
            // Query term proximity ranking
            if self.searcher_config.searcher_options.use_query_term_proximity {
                pl_its_for_proximity_ranking.extend(
                    pl_its
                        .iter()
                        .filter(|pl_it| {
                            pl_it.pl.include_in_proximity_ranking && pl_it.td.unwrap().doc_id == curr_doc_id
                        })
                        .map(|pl_it| pl_it as *const PlIterator),
                );

                if pl_its_for_proximity_ranking.len() > 1 {
                    unsafe {
                        pl_its_for_proximity_ranking
                            .sort_by(|a, b| (**a).original_idx.cmp(&(**b).original_idx));
                    }

                    let num_pl_its_float = pl_its_for_proximity_ranking.len() as f32;

                    let mut position_heap: BinaryHeap<Position> = BinaryHeap::new();
                    for (i, pl_it) in pl_its_for_proximity_ranking.iter().enumerate() {
                        let curr_fields = unsafe {
                            &(**pl_it).td.as_ref().unwrap().fields
                        };
                        for (j, curr_field) in curr_fields.iter().enumerate() {
                            if curr_field.field_positions.is_empty() {
                                continue;
                            }
                            position_heap.push(Position {
                                pos: curr_field.field_positions[0],
                                pl_it_idx: i,
                                pl_it_field_idx: j,
                                pl_it_field_fieldposition_next_idx: 1,
                            });
                        }
                    }

                    // Merge disjoint fields' positions into one
                    // Vec<(pos, pl_it_idx)>
                    let mut merged_positions: Vec<(u32, usize)> = Vec::new();
                    while !position_heap.is_empty() {
                        let top = position_heap.pop().unwrap();

                        let doc_field = unsafe {
                            &(*pl_its_for_proximity_ranking[top.pl_it_idx]).td.as_ref().unwrap().fields[top.pl_it_field_idx]
                        };
                        if top.pl_it_field_fieldposition_next_idx < doc_field.field_positions.len() {
                            position_heap.push(Position {
                                pos: doc_field.field_positions[top.pl_it_field_fieldposition_next_idx],
                                pl_it_idx: top.pl_it_idx,
                                pl_it_field_idx: top.pl_it_field_idx,
                                pl_it_field_fieldposition_next_idx: top.pl_it_field_fieldposition_next_idx + 1,
                            });
                        }

                        merged_positions.push((top.pos, top.pl_it_idx));
                    }

                    let mut next_expected = 0;
                    let mut min_window_len = std::u32::MAX;
                    let mut min_pos = std::u32::MAX;
                    for (pos, pl_it_idx) in merged_positions {
                        if pl_it_idx == 0 {
                            // (Re)start the match from 1
                            min_pos = pos;
                            next_expected = 1;
                        } else if next_expected == pl_it_idx {
                            // Continue the match
                            next_expected += 1;
                        } else {
                            // Restart the match from 0
                            next_expected = 0;
                        }

                        if next_expected >= pl_its_for_proximity_ranking.len() {
                            next_expected = 0;
                            let curr_window_len = pos - min_pos;
                            if curr_window_len < min_window_len {
                                min_window_len = curr_window_len;
                                // web_sys::console::log_1(&format!("min window len {} {} {}", min_window_len, pos, min_pos).into());
                            }
                        }
                    }

                    if min_window_len < 300 {
                        min_window_len += 1;
                        scaling_factor = 1.0
                            + (proximity_ranking_max_scale / (total_proximity_ranking_terms + min_window_len as f32))
                                * (num_pl_its_float / total_proximity_ranking_terms);
                    }
                }

                pl_its_for_proximity_ranking.clear();
            }
            // ------------------------------------------

            // ------------------------------------------
            // BM25 calculation

            for pl_it in pl_its.iter_mut() {
                let td = pl_it.td.unwrap();
                if td.doc_id == curr_doc_id {
                    result.score += if td.score != 0.0 {
                        td.score
                    } else {
                        self.calc_doc_bm25_score(td, curr_doc_id, pl_it.pl)
                    };

                    pl_it.next();
                }
            }

            result.score *= scaling_factor;
            result_heap.push(result);

            if pl_its.iter().any(|pl_it| pl_it.td.is_none()) {
                pl_its = pl_its.into_iter().filter(|pl_it| pl_it.td.is_some()).collect();
            }
            pl_its.sort();

            // ------------------------------------------
        }

        Query {
            searched_terms,
            query_parts,
            result_heap,
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
        static MAJOR_FIELD_FACTOR: f32 = 0.7;
        static MINOR_FIELD_FACTOR: f32 = 0.3;

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
