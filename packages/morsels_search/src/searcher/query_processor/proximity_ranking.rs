use std::{cmp::Ordering, collections::BinaryHeap};

use crate::{postings_list::PlIterator, utils};

pub struct Position {
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

#[inline]
pub fn rank<'a>(
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
                    if pl_it.include_in_proximity_ranking
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
