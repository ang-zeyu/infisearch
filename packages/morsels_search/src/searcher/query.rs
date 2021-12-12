use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::rc::Rc;

use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::postings_list::PlIterator;
use crate::postings_list::PostingsList;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

#[derive(Serialize)]
struct DocResult(u32, f32);

impl Eq for DocResult {}

impl PartialEq for DocResult {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Ord for DocResult {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.1 < other.1 {
            Ordering::Less
        } else if self.1 > other.1 {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for DocResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.1 < other.1 {
            Some(Ordering::Less)
        } else if self.1 > other.1 {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
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
    searched_terms: Vec<String>,
    query_parts: Vec<QueryPart>,
    pub is_free_text_query: bool,
    result_heap: BinaryHeap<DocResult>,
    wand_leftovers: Vec<u32>,
    did_dedup_wand: bool,
}

#[wasm_bindgen]
impl Query {
    pub fn get_next_n(&mut self, n: usize) -> JsValue {
        let mut doc_ids: Vec<DocResult> = Vec::with_capacity(n);
        while !self.result_heap.is_empty() && doc_ids.len() < n {
            doc_ids.push(self.result_heap.pop().unwrap());
        }

        while !self.wand_leftovers.is_empty() && doc_ids.len() < n {
            if !self.did_dedup_wand {
                self.did_dedup_wand = true;
                self.wand_leftovers.sort_unstable();
                self.wand_leftovers.dedup();
            }
            doc_ids.push(DocResult(self.wand_leftovers.pop().unwrap(), 0.0));
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
        n: usize,
        searched_terms: Vec<String>,
        query_parts: Vec<QueryPart>,
        postings_lists: Vec<Rc<PostingsList>>,
        is_free_text_query: bool,
    ) -> Query {
        let mut result_heap: BinaryHeap<DocResult> = BinaryHeap::new();
        let mut top_n_min_heap: BinaryHeap<Reverse<DocResult>> = BinaryHeap::new();
        let mut wand_leftovers: Vec<u32> = Vec::new();

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
            let mut pivot_doc_id = pl_its.first().unwrap().td.unwrap().doc_id;

            // ------------------------------------------
            // WAND
            if is_free_text_query && top_n_min_heap.len() >= n {
                let nth_highest_score = top_n_min_heap.peek().unwrap().0 .1;
                let mut wand_acc = 0.0;
                let mut pivot_list_idx = 0;

                for pl_it in pl_its.iter() {
                    wand_acc += pl_it.pl.max_term_score;
                    if wand_acc > nth_highest_score {
                        pivot_doc_id = pl_it.td.unwrap().doc_id;
                        break;
                    }

                    pivot_list_idx += 1;
                }

                if wand_acc < nth_highest_score {
                    break;
                }

                // Forward pls before the pivot list
                for curr_it in pl_its.iter_mut().take(pivot_list_idx) {
                    while let Some(term_doc) = curr_it.td {
                        if term_doc.doc_id < pivot_doc_id {
                            wand_leftovers.push(term_doc.doc_id);
                            curr_it.next();
                        } else {
                            break;
                        }
                    }
                }

                if pl_its.iter().any(|pl_it| pl_it.td.is_none()) {
                    pl_its = pl_its.into_iter().filter(|pl_it| pl_it.td.is_some()).collect();
                }
            }
            // ------------------------------------------

            let mut result = DocResult(pivot_doc_id, 0.0);
            let mut scaling_factor = 1.0;

            // ------------------------------------------
            // Query term proximity ranking
            if self.searcher_config.searcher_options.use_query_term_proximity {
                pl_its_for_proximity_ranking.extend(
                    pl_its
                        .iter()
                        .filter(|pl_it| {
                            pl_it.pl.include_in_proximity_ranking && pl_it.td.unwrap().doc_id == pivot_doc_id
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
                if td.doc_id == pivot_doc_id {
                    let mut doc_term_score = 0.0;

                    for (field_id, field) in td.fields.iter().enumerate() {
                        if field.field_tf > 0.0 {
                            let field_info = self.searcher_config.field_infos.get(field_id).unwrap();
                            let field_len_factor =
                                self.doc_info.doc_length_factors[pivot_doc_id as usize][field_id as usize] as f32;

                            doc_term_score += ((field.field_tf * (field_info.k + 1.0))
                                / (field.field_tf
                                    + field_info.k * (1.0 - field_info.b + field_info.b * field_len_factor)))
                                * field_info.weight;
                        }
                    }

                    doc_term_score *= pl_it.pl.idf as f32 * pl_it.pl.weight;
                    result.1 += doc_term_score;

                    pl_it.next();
                }
            }

            if top_n_min_heap.len() < n {
                top_n_min_heap.push(Reverse(DocResult(result.0, result.1)));
            } else if result.1 > top_n_min_heap.peek().unwrap().0 .1 {
                top_n_min_heap.pop();
                top_n_min_heap.push(Reverse(DocResult(result.0, result.1)));
            }

            result.1 *= scaling_factor;
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
            is_free_text_query,
            result_heap,
            wand_leftovers,
            did_dedup_wand: false,
        }
    }
}
