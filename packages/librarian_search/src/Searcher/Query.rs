
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use crate::PostingsList::PlIterator;
use crate::Searcher::Searcher;
use crate::PostingsList::PostingsList;
use crate::Searcher::query_parser::QueryPart;

use serde::Serialize;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use wasm_bindgen::prelude::{wasm_bindgen};

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
            Option::from(Ordering::Less)
        } else if self.1 > other.1 {
            Option::from(Ordering::Greater)
        } else {
            Option::from(Ordering::Equal)
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
        if self.pos < other.pos {
            Ordering::Less
        } else if self.pos == other.pos {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.pos < other.pos {
            Option::from(Ordering::Less)
        } else if self.pos == other.pos {
            Option::from(Ordering::Equal)
        } else {
            Option::from(Ordering::Greater)
        }
    }
}

#[wasm_bindgen]
pub struct Query {
    aggregated_terms: Vec<String>,
    query_parts: Vec<QueryPart>,
    pub is_free_text_query: bool,
    result_heap: BinaryHeap<DocResult>,
    wand_leftovers: Vec<u32>,
}

#[wasm_bindgen]
impl Query {
    pub fn get_next_n(&mut self, n: usize) -> JsValue {
        let mut doc_ids: Vec<DocResult> = Vec::with_capacity(n);
        while !self.result_heap.is_empty() && doc_ids.len() < n {
            doc_ids.push(self.result_heap.pop().unwrap());
        }

        while !self.wand_leftovers.is_empty() && doc_ids.len() < n {
            doc_ids.push(DocResult(self.wand_leftovers.pop().unwrap(), 0.0));
        }

        JsValue::from_serde(&doc_ids).unwrap()
    }

    pub fn get_query_parts(&self) -> JsValue {
        JsValue::from_serde(&self.query_parts).unwrap()
    }

    pub fn get_aggregated_terms(&self) -> JsValue {
        JsValue::from_serde(&self.aggregated_terms).unwrap()
    }
}

impl Searcher {
    pub fn create_query(
        &self,
        n: usize,
        aggregated_terms: Vec<String>,
        query_parts: Vec<QueryPart>,
        postings_lists: Vec<Rc<PostingsList>>,
        is_free_text_query: bool,
    ) -> Query {
        let mut result_heap: BinaryHeap<DocResult> = BinaryHeap::new();
        let mut top_n_min_heap: BinaryHeap<DocResult> = BinaryHeap::new();
        let mut wand_leftovers: HashSet<u32> = HashSet::default();

        let mut pl_its: Vec<PlIterator> = postings_lists
            .iter()
            .enumerate()
            .map(|(idx, pl)| pl.get_it(idx as u8))
            .filter(|pl_it| pl_it.td.is_some())
            .collect();
        pl_its.sort();

        let total_proximity_ranking_terms = postings_lists
            .iter()
            .filter(|pl| pl.include_in_proximity_ranking)
            .count() as f32;

        while pl_its.len() > 0 {
            let mut pivot_doc_id = pl_its.get(0).unwrap().td.unwrap().doc_id;

            // ------------------------------------------
            // WAND
            if is_free_text_query && top_n_min_heap.len() >= n {
                let nth_highest_score = top_n_min_heap.peek().unwrap().1;
                let mut wand_acc = 0.0;
                let mut pivot_list_idx = 0;

                while pivot_list_idx < pl_its.len() {
                    wand_acc += pl_its[pivot_list_idx].pl.term_info.as_ref().unwrap().max_term_score;
                    if wand_acc > nth_highest_score {
                        pivot_doc_id = pl_its[pivot_list_idx].td.unwrap().doc_id;
                        break;
                    }

                    pivot_list_idx += 1;
                }

                if wand_acc < nth_highest_score {
                    break;
                }

                // Forward pls before the pivot list
                for i in 0..pivot_list_idx {
                    let curr_it = pl_its.get_mut(i).unwrap();
                    loop {
                        if let Some(term_doc) = curr_it.td {
                            if term_doc.doc_id < pivot_doc_id {
                                // wand_leftovers.insert(term_doc.doc_id);
                                curr_it.next();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }

                pl_its = pl_its.into_iter().filter(|pl_it| pl_it.td.is_some()).collect();
            }
            // ------------------------------------------

            let mut result = DocResult(pivot_doc_id, 0.0);
            let mut scaling_factor = 1.0;

            // ------------------------------------------
            // Query term proximity ranking
            if self.searcher_options.use_query_term_proximity {
                let mut pl_its_for_proximity_ranking: Vec<&mut PlIterator> = pl_its
                    .iter_mut()
                    .filter(|pl_it| pl_it.pl.include_in_proximity_ranking && pl_it.td.unwrap().doc_id == pivot_doc_id)
                    .collect();
                pl_its_for_proximity_ranking.sort_by(|a, b| a.original_idx.cmp(&b.original_idx));

                if pl_its_for_proximity_ranking.len() > 1 {
                    let mut position_heap: BinaryHeap<Position> = BinaryHeap::new();
                    for i in 0..pl_its_for_proximity_ranking.len() {
                        let curr_fields = &pl_its_for_proximity_ranking[i].td.as_ref().unwrap().fields;
                        for j in 0..curr_fields.len() {
                            if curr_fields[j].field_positions.len() == 0 {
                                continue;
                            }
                            position_heap.push(Position {
                                pos: curr_fields[j].field_positions[0],
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

                        let doc_field = &pl_its_for_proximity_ranking[top.pl_it_idx]
                            .td.as_ref().unwrap()
                            .fields[top.pl_it_field_idx];
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
                    let mut curr_window: Vec<u32> = vec![0; pl_its_for_proximity_ranking.len()];
                    for merged_position in merged_positions {
                        if next_expected == merged_position.1 {
                            // Continue the match
                            curr_window[next_expected] = merged_position.0;
                            next_expected += 1;
                        } else if next_expected != 0 && merged_position.1 == 0 {
                            // Restart the match from 1
                            curr_window[0] = merged_position.0;
                            next_expected = 1;
                        } else {
                            // Restart the match from 0
                            next_expected = 0;
                        }

                        if next_expected >= pl_its_for_proximity_ranking.len() {
                            next_expected = 0;
                            min_window_len = std::cmp::min(
                                curr_window.iter().max().unwrap() - curr_window.iter().min().unwrap(),
                                min_window_len
                            );
                        }
                    }

                    if min_window_len < 1000 {
                        scaling_factor = 1.0 + (7.0 / (10.0 + min_window_len as f32))
                            * (pl_its_for_proximity_ranking.len() as f32 / total_proximity_ranking_terms);
                    }
                }
            }
            // ------------------------------------------

            // ------------------------------------------
            // Okapi calculation

            for pl_it in pl_its.iter_mut() {
                let td = pl_it.td.unwrap();
                if td.doc_id == pivot_doc_id {
                    let mut doc_term_score = 0.0;

                    for (field_id, field) in td.fields.iter().enumerate() {
                        if field.field_tf > 0.0 {
                            let field_info = self.field_infos.get(field_id).unwrap();
                            let field_len_factor = self.doc_info.doc_length_factors
                                [pivot_doc_id as usize]
                                [field_id as usize] as f32;
                            
                            doc_term_score += ((field.field_tf * (field_info.k + 1.0))
                                / (field.field_tf + field_info.k * (1.0 - field_info.b + field_info.b * field_len_factor)))
                                * field_info.weight;
                        }
                    }

                    doc_term_score *= pl_it.pl.idf as f32 * pl_it.pl.weight;
                    result.1 += doc_term_score;

                    pl_it.next();
                }
            }

            if top_n_min_heap.len() < n {
                top_n_min_heap.push(DocResult(result.0, result.1));
            } else if result.1 > top_n_min_heap.peek().unwrap().1 {
                top_n_min_heap.pop();
                top_n_min_heap.push(DocResult(result.0, result.1));
            }

            result.1 *= scaling_factor;
            result_heap.push(result);

            pl_its = pl_its.into_iter()
                .filter(|pl_it| pl_it.td.is_some())
                .collect();
            pl_its.sort();

            // ------------------------------------------
        }

        Query {
            aggregated_terms,
            query_parts,
            is_free_text_query,
            result_heap,
            wand_leftovers: wand_leftovers.into_iter().collect(),
        }
    }
}
