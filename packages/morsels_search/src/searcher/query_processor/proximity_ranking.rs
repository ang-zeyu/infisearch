use std::{cmp::Ordering, collections::BinaryHeap};

use crate::{postings_list::{PlIterator, Doc, Field}};

pub struct Position {
    pos: u32,
    pl_it_idx: usize,
    pl_it_field_idx: usize,
    pl_it_field_position_idx: usize,
    pl_it_field_positions: *const Vec<u32>,
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
        Some(self.cmp(other))
    }
}

#[inline]
pub fn rank<'a>(
    is_phrase: bool,
    max_window_len: u32,
    num_scored_fields: usize,
    pl_its: &[PlIterator<'a>],
    proximity_scaling: f32,
    position_heap: &mut BinaryHeap<Position>,
    curr_doc_id: u32,
    total_proximity_ranking_pls: usize,
    min_proximity_ranking_pls: usize,
    scaling_factor: &mut f32,
) -> Option<Doc> {
    const MISSED_TERMS_PENALTY: usize = 4;  // penalty for gaps in terms
    const PROXIMITY_SATURATION: f32 = 4.0;  // how fast it flattens to 1.0

    let mut min_window_len = std::u32::MAX;
    let mut phrase_query_res: Option<Doc> = None;

    debug_assert!(position_heap.is_empty());

    for (pl_it_idx, pl_it) in pl_its.iter().filter(|pl_it| {
        if let Some(prev_td) = pl_it.prev_td {
            if pl_it.include_in_proximity_ranking
                && (!is_phrase || pl_it.is_mandatory)
                && prev_td.doc_id == curr_doc_id {
                return true;
            }
        }
        false
    }).enumerate() {
        let curr_fields = unsafe {
            // prev_td unwrap_unchecked guaranteed by filter earlier
            &*pl_it.prev_td.as_ref().unwrap_unchecked().fields
        };

        for (j, curr_field) in curr_fields.iter().enumerate() {
            if let Some(&pos) = curr_field.field_positions.first() {
                position_heap.push(Position {
                    pos,
                    pl_it_idx,
                    pl_it_field_idx: j,
                    pl_it_field_position_idx: 0,
                    pl_it_field_positions: &curr_field.field_positions,
                });
            }
        }
    }

    let mut next_expected = std::usize::MAX;
    let mut min_pos = std::u32::MAX;
    let mut min_pl_it_field_idx = std::usize::MAX;
    let mut min_terms_missed = total_proximity_ranking_pls - min_proximity_ranking_pls;
    let mut terms_missed = 0;
    while let Some(mut top) = position_heap.pop() {
        while let Some(t) = position_heap.peek() {
            if top.pos == t.pos {
                let mut t = unsafe { position_heap.pop().unwrap_unchecked() };
                if t.pl_it_idx == next_expected {
                    // Use the one that is supposed to fall exactly next,
                    // if any, for phrase queries
                    std::mem::swap(&mut t, &mut top);
                }

                forward_pos(t, position_heap);
            } else {
                break;
            }
        }

        if top.pl_it_idx < next_expected {
            // (Re)start the match from this pl_it
            min_pos = top.pos;
            min_pl_it_field_idx = top.pl_it_field_idx;
            terms_missed = top.pl_it_idx;
            next_expected = top.pl_it_idx + 1;
        } else {
            // Continue the match
            terms_missed += top.pl_it_idx - next_expected;
            next_expected = top.pl_it_idx + 1;

            let curr_window_len = top.pos - min_pos;
            let terms_missed = terms_missed + (total_proximity_ranking_pls - next_expected);
            if terms_missed < min_terms_missed {
                min_terms_missed = terms_missed;
                min_window_len = curr_window_len;
            } else if terms_missed == min_terms_missed && curr_window_len < min_window_len {
                min_window_len = curr_window_len;
                // #[cfg(feature="perf")]
                // web_sys::console::log_1(&format!("min window len {} {} {}", min_window_len, pos, min_pos).into());
            }

            if is_phrase && terms_missed == 0 && curr_window_len == max_window_len {
                if phrase_query_res.is_none() {
                    phrase_query_res = Some(Doc {
                        doc_id: curr_doc_id,
                        fields: vec![
                            Field {
                                field_tf: 0.0,
                                field_positions: Vec::new(),
                            };
                            num_scored_fields
                        ],
                        score: 0.0,
                    })
                }

                let fields = &mut unsafe { phrase_query_res.as_mut().unwrap_unchecked() }.fields;
                let field = unsafe { fields.get_unchecked_mut(min_pl_it_field_idx) };
                field.field_positions.push(min_pos);
                field.field_tf += 1.0;
            }
        }

        forward_pos(top, position_heap);
    }

    if min_window_len <= max_window_len {
        // TODO make this non-linear? (caps off at certain degree)
        min_window_len *= 1 + (min_terms_missed * MISSED_TERMS_PENALTY) as u32;

        if min_window_len <= max_window_len {
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
                &format!("+ min_window_len {} min_terms_missed {} scaling_factor {}",
                min_window_len, min_terms_missed, scaling_factor,
            ).into()); */
        } else {
            /* #[cfg(feature="perf")]
            web_sys::console::log_1(
                &format!("- min_window_len {} min_terms_missed {} scaling_factor {}",
                min_window_len, min_terms_missed, scaling_factor,
            ).into()); */
        }
    }

    return phrase_query_res;
}

fn forward_pos(
    mut top: Position,
    position_heap: &mut BinaryHeap<Position>,
) {
    // Update Position iterator
    top.pl_it_field_position_idx += 1;
    if let Some(&pos) = unsafe { &*top.pl_it_field_positions }.get(top.pl_it_field_position_idx) {
        top.pos = pos;
        position_heap.push(top);
    }
}
