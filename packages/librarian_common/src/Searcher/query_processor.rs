use std::cmp::Reverse;
use crate::PostingsList::DocField;
use std::cell::RefCell;
use crate::PostingsList::TermDoc;
use std::collections::BinaryHeap;
use crate::PostingsList::PlIterator;
use std::rc::Rc;
use crate::PostingsList::PostingsList;
use rustc_hash::FxHashMap;
use crate::Searcher::query_parser::QueryPart;
use crate::Searcher::query_parser::QueryPartType;
use crate::Searcher::Searcher;

impl Searcher {
    fn populate_phrasal_postings_lists(
        &self,
        query_part: &QueryPart,
        term_postings_lists: &FxHashMap<String, Rc<PostingsList>>
    ) -> Rc<PostingsList> {
        let pl_iterators: Vec<Rc<RefCell<PlIterator>>> = query_part.terms.as_ref().unwrap()
            .iter()
            .enumerate()
            .map(|(idx, term)| {
                Rc::new(RefCell::new(term_postings_lists.get(term).unwrap().get_it(idx as u8)))
            })
            .collect();
        let mut iterator_heap: BinaryHeap<Reverse<Rc<RefCell<PlIterator>>>> = pl_iterators.iter()
            .map(|pl_it| Reverse(Rc::clone(pl_it)))
            .collect();
        let num_pls = iterator_heap.len();
        
        let mut result_pl = PostingsList {
            weight: 1.0,
            include_in_proximity_ranking: true,
            term_docs: Vec::new(),
            idf: 0.0,
            term: Option::None,
            term_info: Option::None,
        };

        let mut curr_doc_id = self.doc_info.num_docs + 1;
        let mut curr_num_docs = 0;
        while !iterator_heap.is_empty() {
            let min_pl_iterator_rc = iterator_heap.pop().unwrap();
            let mut min_pl_iterator = min_pl_iterator_rc.0.borrow_mut();

            if min_pl_iterator.td.unwrap().doc_id == curr_doc_id {
                curr_num_docs += 1;

                if min_pl_iterator.next().is_some() {
                    drop(min_pl_iterator);
                    iterator_heap.push(min_pl_iterator_rc);
                } else {
                    drop(min_pl_iterator);
                }

                if curr_num_docs != num_pls {
                    continue;
                }

                let mut td = TermDoc {
                    doc_id: curr_doc_id,
                    fields: Vec::new(),
                };
                let mut has_match = false;

                let term_termdocs: Vec<_> = pl_iterators.iter()
                    .map(|pl_it| pl_it.borrow().peek_prev().unwrap())
                    .collect();

                for field_id in 0..self.num_scored_fields as u8 {
                    let mut result_doc_field = DocField {
                        field_id,
                        field_positions: Vec::new(),
                    };

                    let mut term_field_position_idxes = vec![0; num_pls];
                    let mut curr_pos: u32 = 0;
                    let mut term_idx = 0;
                    loop {
                        let curr_term_termdocs = *term_termdocs.get(term_idx).unwrap();
                        if let Some(curr_pl_field) = curr_term_termdocs.fields.get(field_id as usize) {
                            if let Some(pos) = curr_pl_field.field_positions.get(term_field_position_idxes[term_idx]) {
                                if term_idx == 0 {
                                    term_field_position_idxes[term_idx] += 1;

                                    curr_pos = *pos;
                                    term_idx += 1;
                                } else if *pos == (curr_pos + 1) {
                                    term_field_position_idxes[term_idx] += 1;

                                    if term_idx == num_pls - 1 {
                                        // Complete the match
                                        has_match = true;
                                        result_doc_field.field_positions.push(*pos - (num_pls as u32) + 1);
                                        
                                        // Reset
                                        term_idx = 0;
                                    } else {
                                        // Match next term
                                        curr_pos = *pos;
                                        term_idx += 1;
                                    }
                                } else {
                                    // Not matched

                                    // Forward this postings list up to currPos, try again
                                    if *pos < curr_pos {
                                        while term_field_position_idxes[term_idx] < curr_pl_field.field_positions.len()
                                            && curr_pl_field.field_positions[term_field_position_idxes[term_idx]] < curr_pos {
                                            term_field_position_idxes[term_idx] += 1;
                                        }
                                        continue;
                                    }

                                    // Reset
                                    term_idx = 0;
                                }
                            } else {
                                // exceeded number of positions
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    td.fields.push(result_doc_field);
                }

                curr_doc_id = self.doc_info.num_docs + 1;
                curr_num_docs = 0;

                if has_match {
                    result_pl.term_docs.push(td);
                }
            } else {
                curr_doc_id = min_pl_iterator.td.unwrap().doc_id;
                curr_num_docs = 1;

                if min_pl_iterator.next().is_some() {
                    drop(min_pl_iterator);
                    iterator_heap.push(min_pl_iterator_rc);
                }
            }
        }

        result_pl.calc_pseudo_idf(self.doc_info.num_docs);

        Rc::new(result_pl)
    }

    fn populate_and_postings_lists(
        &self,
        query_part: &mut QueryPart,
        term_postings_lists: &FxHashMap<String, Rc<PostingsList>>
    ) -> Rc<PostingsList> {
        let pl_vecs = self.populate_postings_lists(query_part.children.as_mut().unwrap(), term_postings_lists);
        
        let mut doc_heap: BinaryHeap<Reverse<PlIterator>> = pl_vecs
            .iter()
            .enumerate()
            .map(|(idx, pl_vec)| Reverse(pl_vec.get_it(idx as u8)))
            .filter(|pl_it| pl_it.0.td.is_some())
            .collect();
        let num_pls = doc_heap.len();

        let mut result_pl = PostingsList {
            weight: 1.0,
            include_in_proximity_ranking: true,
            term_docs: Vec::new(),
            idf: 0.0,
            term: Option::None,
            term_info: Option::None,
        };

        let mut curr_doc_id: u32 = self.doc_info.num_docs + 1;
        let mut curr_num_docs = 0;
        while !doc_heap.is_empty() {
            let mut min_pl_iterator = doc_heap.pop().unwrap();

            if min_pl_iterator.0.td.unwrap().doc_id == curr_doc_id {
                if min_pl_iterator.0.next().is_some() {
                    doc_heap.push(min_pl_iterator);
                }

                curr_num_docs += 1;

                if curr_num_docs == num_pls {
                    let mut acc = TermDoc {
                        doc_id: curr_doc_id,
                        fields: Vec::new(),
                    };
                    for td in doc_heap.iter().map(|pl_it| pl_it.0.peek_prev().unwrap()) {
                        acc = PostingsList::merge_term_docs(td, &acc);
                    }
                    result_pl.term_docs.push(acc);
                    
                    curr_doc_id = self.doc_info.num_docs + 1;
                    curr_num_docs = 0;
                }
            } else {
                curr_doc_id = min_pl_iterator.0.td.unwrap().doc_id;
                curr_num_docs = 1;

                if min_pl_iterator.0.next().is_some() {
                    doc_heap.push(min_pl_iterator);
                }
            }
        }

        result_pl.calc_pseudo_idf(self.doc_info.num_docs);

        Rc::new(result_pl)
    }

    fn populate_not_postings_list(
        &self,
        query_part: &mut QueryPart,
        term_postings_lists: &FxHashMap<String, Rc<PostingsList>>
    ) -> Rc<PostingsList> {
        let not_child_postings_list = self.populate_postings_lists(
            query_part.children.as_mut().unwrap(),
            term_postings_lists,
        ).remove(0);
        let mut result_pl = PostingsList {
            weight: 1.0,
            include_in_proximity_ranking: false,
            term_docs: Vec::new(),
            idf: 0.0,
            term: Option::None,
            term_info: Option::None,
        };

        let mut prev = 0;
        for td in not_child_postings_list.term_docs.iter() {
            for doc_id in prev..td.doc_id {
                result_pl.term_docs.push(TermDoc {
                    doc_id,
                    fields: Vec::new(),
                });
            }
            prev = td.doc_id + 1;
        }
        for doc_id in prev..self.doc_info.num_docs {
            result_pl.term_docs.push(TermDoc {
                doc_id,
                fields: Vec::new(),
            });
        }

        result_pl.calc_pseudo_idf(self.doc_info.num_docs);

        Rc::new(result_pl)
    }

    fn populate_bracket_postings_list(
        &self,
        query_part: &mut QueryPart,
        term_postings_lists: &FxHashMap<String, Rc<PostingsList>>
    ) -> Option<Rc<PostingsList>> {
        let mut child_postings_lists = self.populate_postings_lists(
            query_part.children.as_mut().unwrap(),
            term_postings_lists,
        );

        if child_postings_lists.len() == 0 {
            return Option::None;
        } else if child_postings_lists.len() == 1 {
            return Option::from(child_postings_lists.pop().unwrap());
        }
        
        let mut doc_heap: BinaryHeap<Reverse<PlIterator>> = child_postings_lists
            .iter()
            .enumerate()
            .map(|(idx, pl_vec)| Reverse(pl_vec.get_it(idx as u8)))
            .filter(|pl_it| pl_it.0.td.is_some())
            .collect();
        let num_pls = doc_heap.len();

        let mut new_pl = PostingsList {
            weight: 1.0,
            include_in_proximity_ranking: true,
            term_docs: Vec::new(),
            idf: 0.0,
            term: Option::None,
            term_info: Option::None,
        };

        let mut curr_doc_id: u32 = self.doc_info.num_docs + 1;
        let mut curr_pl_iterators: Vec<Reverse<PlIterator>> = Vec::with_capacity(num_pls);

        let mut merge_curr_termdocs = |curr_pl_iterators: &Vec<Reverse<PlIterator>>| {
            let initial_td = curr_pl_iterators[curr_pl_iterators.len() - 1].0.td.unwrap().clone();
            let merged_term_docs = curr_pl_iterators.iter()
                .fold(
                    initial_td,
                    |acc, next| PostingsList::merge_term_docs(&acc, next.0.td.unwrap())
                );
            new_pl.term_docs.push(merged_term_docs);
        };

        while !doc_heap.is_empty() {
            let min_pl_iterator = doc_heap.pop().unwrap();
            let min_pl_iterator_doc_id = min_pl_iterator.0.td.unwrap().doc_id;

            if min_pl_iterator_doc_id != curr_doc_id && curr_pl_iterators.len() > 0 {
                merge_curr_termdocs(&curr_pl_iterators);

                for mut curr_pl_it in curr_pl_iterators.drain(..) {
                    if curr_pl_it.0.next().is_some() {
                        doc_heap.push(curr_pl_it);
                    }
                }

                curr_doc_id = min_pl_iterator_doc_id;
            }

            curr_pl_iterators.push(min_pl_iterator);
        }
        
        if curr_pl_iterators.len() > 0 {
            merge_curr_termdocs(&curr_pl_iterators);
        }

        new_pl.calc_pseudo_idf(self.doc_info.num_docs);

        Option::from(Rc::new(new_pl))
    }

    fn populate_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: &FxHashMap<String, Rc<PostingsList>>
    ) -> Vec<Rc<PostingsList>> {
        let mut result: Vec<Rc<PostingsList>> = Vec::new();

        for query_part in query_parts {
            match query_part.typee {
                QueryPartType::TERM => {
                    if let Some(term) = query_part.terms.as_ref().unwrap().get(0) {
                        if let Some(term_pl) = term_postings_lists.get(term) {
                            result.push(Rc::clone(term_pl));
                        }
                    }
                },
                QueryPartType::PHRASE => {
                    result.push(self.populate_phrasal_postings_lists(query_part, term_postings_lists));
                },
                QueryPartType::AND => {
                    result.push(self.populate_and_postings_lists(query_part, term_postings_lists));
                },
                QueryPartType::NOT => {
                    result.push(self.populate_not_postings_list(query_part, term_postings_lists));
                },
                QueryPartType::BRACKET => {
                    if let Some(bracket_postings_list) = self.populate_bracket_postings_list(query_part, term_postings_lists) {
                        result.push(bracket_postings_list);
                    }
                }
                _ => {}
            }
        }

        result
    }

    pub fn process(
        &self,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: FxHashMap<String, PostingsList>
    ) -> Vec<Rc<PostingsList>> {
        let term_rc_postings_lists: FxHashMap<String, Rc<PostingsList>> = term_postings_lists
            .into_iter()
            .map(|(term, pl)| (term, Rc::new(pl)))
            .collect();
        
        self.populate_postings_lists(query_parts, &term_rc_postings_lists)
    }
}