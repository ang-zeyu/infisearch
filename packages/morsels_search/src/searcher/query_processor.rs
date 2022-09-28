mod bm25;
mod proximity_ranking;

use std::collections::BinaryHeap;
use std::rc::Rc;

use morsels_common::bitmap;

use crate::postings_list::{self, Field, PlIterator, PostingsList, Doc, PlAndInfo};
use crate::searcher::query_parser::QueryPart;
use crate::searcher::query_parser::QueryPartType;
use crate::searcher::Searcher;
use crate::utils;

use super::query::DocResult;


fn empty_pl() -> PostingsList {
    PostingsList {
        term_docs: Vec::new(),
        idf: 0.0,
        term: None,
        term_info: None,
    }
}

impl Searcher {
    fn populate_phrasal_postings_lists(
        &self,
        query_part: &QueryPart,
        term_postings_lists: &Vec<Rc<PostingsList>>,
        weight: f32,
    ) -> Rc<PostingsList> {
        let mut encountered_empty_pl = false;

        // Keep the original ordering for performing the phrase query.
        // The contents can be mutated, but the Vec itself must never be resized / reordered / etc.
        // Otherwise, sorted_pl_its below might point to invalid things...
        let mut pl_iters: Vec<PlIterator> = query_part
            .terms
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(idx, term)| {
                let pl_iter = postings_list::get_postings_list_rc(term, term_postings_lists)
                    .unwrap()
                    .iter(
                        idx as u8,
                        weight,
                        // Unused
                        false,
                        false,
                        false,
                        false,
                    );
                if pl_iter.td.is_none() {
                    encountered_empty_pl = true;
                }

                pl_iter
            })
            .collect();

        let mut result_pl = empty_pl();

        if encountered_empty_pl || pl_iters.is_empty() {
            return Rc::new(result_pl);
        }

        // Avoid Rc<RefCell<...>>
        let mut sorted_pl_its: Vec<*mut PlIterator> = pl_iters
            .iter_mut()
            .map(|pl_it| pl_it as *mut PlIterator)
            .collect();
        let num_pls = sorted_pl_its.len();

        // Local to has_position_match
        let mut term_field_position_idxes = vec![0; num_pls];

        loop {
            utils::insertion_sort(&mut sorted_pl_its, |&a, &b| unsafe {
                (*a).lt(&*b)
            });

            let min_pl_iter = unsafe { &**sorted_pl_its.first().unwrap() };
            if let Some(first_td) = min_pl_iter.td {
                // Do an "AND" query first

                let curr_doc_id = first_td.doc_id;
                let mut num_matched_docs = 0;
                for &pl_it in sorted_pl_its.iter() {
                    let pl_it = unsafe { &mut *pl_it };
                    if let Some(td) = pl_it.td {
                        if td.doc_id == curr_doc_id {
                            pl_it.next();

                            debug_assert!(pl_it.prev_td.is_some());

                            num_matched_docs += 1;
                        }
                    }
                }

                debug_assert!(num_matched_docs > 0);

                if num_matched_docs == num_pls {
                    // Now do the phrase query on curr_doc_id
                    let (td, has_match) = self.has_position_match(
                        curr_doc_id, num_pls, &pl_iters, &mut term_field_position_idxes
                    );

                    if has_match {
                        result_pl.term_docs.push(td);
                    }
                }
            } else {
                break;
            }
        }

        result_pl.calc_pseudo_idf(self.doc_info.num_docs);

        Rc::new(result_pl)
    }

    fn has_position_match(
        &self,
        curr_doc_id: u32, 
        num_pls: usize,
        pl_iters: &Vec<PlIterator>,
        term_field_position_idxes: &mut Vec<usize>,
    ) -> (Doc, bool) {
        let mut td = Doc { doc_id: curr_doc_id, fields: Vec::new(), score: 0.0 };
        let mut has_match = false;
        for field_id in 0..self.searcher_config.num_scored_fields as usize {
            let mut result_doc_field = Field { field_tf: 0.0, field_positions: Vec::new() };

            for v in term_field_position_idxes.iter_mut() { *v = 0; }
            let mut curr_pos: u32 = 0;
            let mut term_idx = 0;

            // Go through the terms in this field, controlled by term_idx modifications below
            while let Some(curr_pl_field) = pl_iters[term_idx].prev_td.unwrap().fields.get(field_id) {
                if let Some(&pos) = curr_pl_field.field_positions.get(term_field_position_idxes[term_idx]) {
                    if term_idx == 0 {
                        // First term in the query
                        term_field_position_idxes[0] += 1;

                        curr_pos = pos;
                        term_idx += 1;
                    } else if pos == (curr_pos + 1) {
                        // Matched the next term
                        term_field_position_idxes[term_idx] += 1;

                        if term_idx == num_pls - 1 {
                            // Complete the match
                            has_match = true;
                            result_doc_field.field_positions.push(pos + 1 - (num_pls as u32));

                            // Reset to look for first term
                            term_idx = 0;
                        } else {
                            // Match next term
                            curr_pos = pos;
                            term_idx += 1;
                        }
                    } else {
                        // Not matched

                        // Forward this postings list up to currPos, try again
                        if pos < curr_pos {
                            while term_field_position_idxes[term_idx] < curr_pl_field.field_positions.len()
                                && curr_pl_field.field_positions[term_field_position_idxes[term_idx]] < curr_pos
                            {
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
            }

            result_doc_field.field_tf = result_doc_field.field_positions.len() as f32;

            td.fields.push(result_doc_field);
        }
        (td, has_match)
    }

    fn populate_conjunctive_postings_lists(
        &self,
        do_accumulate: bool,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: &Vec<Rc<PostingsList>>,
        weight: f32,
    ) -> PostingsList {
        let mut new_pl = empty_pl();
        new_pl.calc_pseudo_idf(self.doc_info.num_docs);

        let child_postings_lists = self.process_pls(
            query_parts,
            term_postings_lists,
            weight,
        );

        if child_postings_lists.is_empty() {
            return new_pl;
        }

        let mut sorted_pl_its: Vec<PlIterator> = child_postings_lists
            .iter()
            .enumerate()
            .map(|(idx, pl_and_info)| pl_and_info.pl.iter(
                idx as u8,
                pl_and_info.weight,
                pl_and_info.include_in_proximity_ranking,
                pl_and_info.is_mandatory,
                pl_and_info.is_subtracted,
                pl_and_info.is_inverted,
            ))
            .collect();

        // ------------------------------------------
        // Query term proximity ranking
        const PROXIMITY_BASE_SCALING: f32 = 2.5;
        const PROXIMITY_PER_TERM_SCALING: f32 = 0.5;

        let total_proximity_ranking_terms = child_postings_lists.iter()
            .filter(|pl_and_info| pl_and_info.include_in_proximity_ranking)
            .count();
        let min_proximity_ranking_terms = ((total_proximity_ranking_terms as f32 / 2.0).ceil() as usize).max(2);
        let proximity_scaling = PROXIMITY_BASE_SCALING
            + (total_proximity_ranking_terms as f32 * PROXIMITY_PER_TERM_SCALING);

        let mut pl_its_for_proximity_ranking: Vec<*const PlIterator> = Vec::with_capacity(sorted_pl_its.len());
        let mut position_heap = BinaryHeap::with_capacity(
            total_proximity_ranking_terms as usize * self.searcher_config.num_scored_fields,
        );
        // ------------------------------------------

        let num_mandatory_pls = sorted_pl_its.iter()
            .filter(|pl_it| pl_it.is_mandatory)
            .count();

        loop {
            utils::insertion_sort(&mut sorted_pl_its, |a, b| a.lt(b));

            let doc_id = if num_mandatory_pls > 0 {
                // Find the largest mandatory id for forwarding other postings lists
                let id = sorted_pl_its
                    .iter()
                    .rev()
                    .find_map(|pl_it| if pl_it.is_mandatory {
                        if let Some(td) = pl_it.td {
                            Some(td.doc_id)
                        } else {
                            // An exhausted, mandatory postings list
                            None
                        }
                    } else {
                        None
                    });

                if let Some(id) = id {
                    id
                } else {
                    break;
                }
            } else if let Some(first_id) = unsafe {
                // guaranteed by .is_empty() check
                sorted_pl_its.get_unchecked(0)
            }.td.map(|td| td.doc_id) {
                first_id
            } else {
                break;
            };

            let mut score = 0.0;
            let mut num_mandatory_pls_matched = 0;
            let mut is_subtracted = false;

            let mut acc = Doc { doc_id, fields: Vec::new(), score: 0.0 };

            for pl_it in sorted_pl_its.iter_mut() {
                while let Some(td) = pl_it.td {
                    if td.doc_id == doc_id {
                        if pl_it.is_subtracted {
                            is_subtracted = true;
                        } else {
                            score += if td.score != 0.0 {
                                td.score
                            } else {
                                self.calc_doc_bm25_score(td, doc_id, pl_it.pl, pl_it.weight)
                            };

                            if pl_it.is_mandatory {
                                num_mandatory_pls_matched += 1;
                            }

                            if do_accumulate {
                                // Skip merging positions, term frequencies for non top-level postings lists
                                acc = PostingsList::merge_term_docs(td, &acc);
                            }
                        }
                    } else if td.doc_id > doc_id {
                        break;
                    }

                    pl_it.next();
                }
            }

            // ------------------------------------------
            // Query term proximity ranking

            let mut positional_scaling_factor = 1.0;
            if self.searcher_config.searcher_options.use_query_term_proximity {
                proximity_ranking::rank(
                    &sorted_pl_its,
                    &mut pl_its_for_proximity_ranking,
                    proximity_scaling,
                    &mut position_heap,
                    doc_id,
                    total_proximity_ranking_terms,
                    min_proximity_ranking_terms,
                    &mut positional_scaling_factor,
                );
            }
            // ------------------------------------------

            if !is_subtracted && !(num_mandatory_pls > 0 && num_mandatory_pls_matched < num_mandatory_pls) {
                acc.score = score * positional_scaling_factor;
                new_pl.term_docs.push(acc);
            }
        }

        new_pl.calc_pseudo_idf(self.doc_info.num_docs);

        new_pl
    }

    fn invert_postings_list(&self, pl: Rc<PostingsList>, weight: f32) -> Rc<PostingsList> {
        let mut result_pl = empty_pl();

        let mut prev = 0;
        for td in pl.term_docs.iter() {
            for doc_id in prev..td.doc_id {
                if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                    result_pl.term_docs.push(Doc { doc_id, fields: Vec::new(), score: 0.0 });
                }
            }
            prev = td.doc_id + 1;
        }

        for doc_id in prev..self.doc_info.doc_length_factors_len {
            if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                result_pl.term_docs.push(Doc { doc_id, fields: Vec::new(), score: 0.0 });
            }
        }

        result_pl.calc_pseudo_idf(self.doc_info.num_docs);

        // Same score for every result. Score resulting from field tf is taken as 1.
        let score = result_pl.idf as f32 * weight;
        for term_doc in result_pl.term_docs.iter_mut() {
            term_doc.score = score;
        }

        Rc::new(result_pl)
    }

    fn filter_field_postings_list(&self, field_name: &str, pl: &mut Rc<PostingsList>, weight: f32) {
        let field_id_and_info = self
            .searcher_config
            .field_infos
            .iter()
            .enumerate()
            .find(|(_id, field_info)| field_info.name == field_name);

        if let Some((field_id, _field_info)) = field_id_and_info {
            let mut new_pl = PostingsList {
                term_docs: Vec::new(),
                idf: pl.idf,
                term: pl.term.clone(),
                term_info: pl.term_info.clone(),
            };

            let fields_before = vec![Field::default(); field_id];
            for term_doc in &pl.term_docs {
                if let Some(doc_field) = term_doc.fields.get(field_id) {
                    if doc_field.field_tf == 0.0 {
                        continue;
                    }

                    let mut fields: Vec<Field> = fields_before.clone();
                    fields.push(doc_field.clone()); // TODO reduce potential allocations?

                    let score = if term_doc.score != 0.0 {
                        term_doc.score
                    } else {
                        self.calc_doc_bm25_score(term_doc, term_doc.doc_id, pl, weight)
                    };
                    new_pl.term_docs.push(Doc { doc_id: term_doc.doc_id, fields, score })
                }
            }

            new_pl.calc_pseudo_idf(self.doc_info.num_docs);
            *pl = Rc::new(new_pl);
        }
    }

    /*
    Processes query operators before the final round in rank_top_level.
    Postings lists are always still in document id order after being processed here.
    (for efficient processing in AND / NOT / () / Phrase operators)

    Scoring:
    - AND / () operators: scores of expressions within are calculated if necessary and summed
    - Field filters: the filtered expression's score is calculated if necessary
    - NOT: the same score is calculated and assigned to every document in the result set
    - Phrase: scoring is delayed until necessary
    */
    fn process_pls(
        &self,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: &Vec<Rc<PostingsList>>,
        weight: f32,
    ) -> Vec<PlAndInfo> {
        let mut result: Vec<PlAndInfo> = Vec::new();


        for query_part in query_parts {
            let mut pl_opt: Option<Rc<PostingsList>> = None;
            let weight = weight * query_part.weight;

            if let Some(children) = &mut query_part.children {
                debug_assert!(query_part.terms.is_none() && matches!(query_part.part_type, QueryPartType::Bracket));

                pl_opt = Some(Rc::new(self.populate_conjunctive_postings_lists(
                    true, children, term_postings_lists, weight,
                )));
            }

            if let Some(terms) = &query_part.terms {
                debug_assert!(query_part.children.is_none() && (
                    matches!(query_part.part_type, QueryPartType::Term)
                    || matches!(query_part.part_type, QueryPartType::Phrase)
                ));

                if terms.len() == 1 {
                    if let Some(term) = terms.first() {
                        if let Some(term_pl) = postings_list::get_postings_list_rc(term, term_postings_lists) {
                            pl_opt = Some(Rc::clone(term_pl));
                        }
                    }
                } else if terms.len() > 1 {
                    debug_assert!(matches!(query_part.part_type, QueryPartType::Phrase));

                    pl_opt = Some(self.populate_phrasal_postings_lists(query_part, term_postings_lists, weight));
                } /* else {
                    spelling correct / stop word removed, ignore
                } */
            }

            let mut pl = pl_opt.unwrap_or(Rc::new(empty_pl()));

            if let Some(field_name) = &query_part.field_name {
                self.filter_field_postings_list(field_name, &mut pl, weight);
            }

            // Negation after field filter. If before, it would just return an empty list.
            if query_part.is_inverted {
                pl = self.invert_postings_list(pl, weight);
            }

            result.push(PlAndInfo {
                pl,
                weight,
                include_in_proximity_ranking: !(query_part.is_suffixed || query_part.is_inverted),
                is_mandatory: query_part.is_mandatory,
                is_subtracted: query_part.is_subtracted,
                is_inverted: query_part.is_inverted,
            })
        }

        result
    }

    pub fn process_and_rank(
        &self,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: &Vec<Rc<PostingsList>>,
    ) -> BinaryHeap<DocResult> {
        let root_pl = self.populate_conjunctive_postings_lists(
            false, query_parts, term_postings_lists, 1.0,
        );

        root_pl.term_docs.into_iter()
            .map(|td| DocResult { doc_id: td.doc_id, score: td.score })
            .collect()
    }
}


#[cfg(test)]
mod test {
    use std::rc::Rc;

    use pretty_assertions::assert_eq;

    use crate::postings_list::test::{to_pl, to_pl_rc};
    use crate::postings_list::PostingsList;
    use crate::searcher::query_parser::test as query_parser_test;
    use crate::searcher::test as searcher_test;

    struct TermPostingsListsBuilder(Vec<PostingsList>);

    impl TermPostingsListsBuilder {
        fn new() -> Self {
            TermPostingsListsBuilder(Vec::new())
        }

        fn with(mut self, term: &str, pl_str: &str) -> Self {
            self.0.push(to_pl(Some(term.to_owned()), pl_str));
            self
        }

        fn get_rc_wrapped(self) -> Vec<Rc<PostingsList>> {
            self.0.into_iter().map(Rc::new).collect()
        }
    }

    fn search(query: &str, term_postings_lists: Vec<Rc<PostingsList>>) -> PostingsList {
        let mut parsed = query_parser_test::parse(query);
        searcher_test::create_searcher(10).populate_conjunctive_postings_lists(
            true,
            false,
            &mut parsed,
            &term_postings_lists,
            1.0,
        )
    }

    fn search_w_sw_removal(query: &str, term_postings_lists: Vec<Rc<PostingsList>>) -> PostingsList {
        let mut parsed = query_parser_test::parse_with_sw_removal(query);
        let mut s = searcher_test::create_searcher(10);
        s.searcher_config.lang_config.options.ignore_stop_words = Some(true);
        s.populate_conjunctive_postings_lists(
            true,
            false,
            &mut parsed,
            &term_postings_lists,
            1.0,
        )
    }

    // See postings_list.rs to_pl for construction format

    #[test]
    fn test_phrasal_queries() {
        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[3,[1,12,31]]]")
                    .with("ipsum", "[[3,[11,13,32]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[2,[12,31]],[0,[]],[0,[]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[0,[]],[3,[1,12,31]]]")
                    .with("ipsum", "[[0,[]],[0,[]],[3,[11,13,32]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[0,[]],[2,[12,31]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null, null, [[0,[]],[0,[]],[3,[1,12,31]]]")
                    .with("ipsum", "null, null, [[0,[]],[0,[]],[3,[11,13,32]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("null, null, [[0,[]],[0,[]],[2,[12,31]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[1,3,5,7]],[0,[]],[0,[]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]], [1,[11]]]")
                    .with("ipsum", "[[4,[2,4,6,8]], [1,[12]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[1,3,5,7]],[1,[11]],[0,[]]]"),
        );

        assert_eq!(
            search(
                "~\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("null, [], [], [], [], [], [], [], [], []"),
        );
    }

    #[test]
    fn test_phrasal_queries_negative() {
        // Different positions
        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[3,[1,12,31]]]")
                    .with("ipsum", "[[3,[11,14,33]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        // Different docs
        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,             [[3,[1,12,31]]]")
                    .with("ipsum", "[[3,[11,13,32]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        // Different fields
        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[3,[1,12,31]]]")
                    .with("ipsum", "[[3,[11,13,32]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null, null, [[0,[]],[3,[1,12,31]],         [0,[]]]")
                    .with("ipsum", "null, null, [[0,[]],[0,[]],[3,[11,13,32]]]        ")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );
    }

    #[test]
    fn test_field_queries() {
        assert_eq!(
            search(
                "heading:lorem",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]], [1,[11]], [2,[65,100]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[0,[]],[2,[65,100]]]"),
        );

        assert_eq!(
            search(
                "title:lorem body:ipsum)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]], [1,[11]]]")
                    .with("ipsum", "[[4,[2,4,6,8]], [1,[12]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[1,3,5,7]], [1,[12]]]"),
        );

        assert_eq!(
            search(
                "title: (lorem ipsum)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]], [1,[11]]]")
                    .with("ipsum", "[[4,[2,4,6,8]], [1,[12]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[8,[1,2,3,4,5,6,7,8]]]"),
        );

        assert_eq!(
            search(
                "title: \"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]], [1,[11]]]")
                    .with("ipsum", "[[4,[2,4,6,8]], [1,[12]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[1,3,5,7]]]"),
        );

        assert_eq!(
            search(
                "body: \"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]], [1,[11]]]")
                    .with("ipsum", "[[4,[2,4,6,8]], [1,[12]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[1,[11]]]"),
        );
    }

    #[test]
    fn test_mandatory_queries() {
        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "")
                    .with("ipsum", "")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        assert_eq!(
            search(
                " +lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]]]")
                    .with("ipsum", "")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        // Different fields still match
        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[2,[1]]]"),
        );

        // Test position, field merging behaviour
        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[1,[2]]]")
                    .with("ipsum", "[[1,[10]],[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[1,[10]],[2,[1,2]]]"),
        );

        // Multiple docs
        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[3,[1,2,8]]], [[0,[]],[1,[1]]], [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]           , null            , [[3,[1,5,9]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[1,[1]],[3,[1,2,8]]], null, [[4,[1,5,9]]]"),
        );

        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], [[2,[1,3]]],        null, [[2,[1,3]]], [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[0,[]],[4,[1,3,5,7]]],        null, [[2,[1,3]]], [[2,[1,3]]], [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[8,[1,3,5,7]]], null, null, [[4,[1,3]]], [[0,[]],[8,[1,3,5,7]]]")
        );

        // With a non-mandatory term
        assert_eq!(
            search(
                "+lorem +ipsum for",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,     [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .with("for", "null, [[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        assert_eq!(
            search(
                "+lorem +ipsum for http",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,     [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .with("for", "[[1,[1]]]")
                    .with("http", "null, [[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );
    }

    #[test]
    fn test_and_queries_negative() {
        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,     [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );
    }

    #[test]
    fn test_freetext_queries() {
        assert_eq!(
            search(
                "lorem ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[2,4,6,8]],[4,[1,3,5,7]]], null, [], [[0,[]],[4,[1,3,5,7]]]"),
        );

        assert_eq!(
            search(
                "lorem lorem",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[8,[1,3,5,7]]], null, null, [[0,[]],[8,[1,3,5,7]]]"),
        );

        // SW removal
        assert_eq!(
            search(
                "for ipsum",
                TermPostingsListsBuilder::new()
                .with("for",   "[[0,[]],[4,[1,3,5,7]]],   [], [[0,[]],[4,[1,3,5,7]]]")
                .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [[0,[]],[4,[1,2,5,9]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[2,4,6,8]],[4,[1,3,5,7]]], [], [[0,[]],[8,[1,2,3,5,7,9]]]"),
        );

        assert_eq!(
            search_w_sw_removal(
                "for ipsum",
                TermPostingsListsBuilder::new()
                    .with("for",   "[[0,[]],[4,[1,3,5,7]]],   [], [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [[0,[]],[4,[1,2,5,9]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[2,4,6,8]],[0,[]]], null, [[0,[]],[4,[1,2,5,9]]]"),
        );
    }

    #[test]
    fn test_parentheses_queries() {
        assert_eq!(
            search(
                "(",
                TermPostingsListsBuilder::new().with("lorem", "[[1,[1]]]").with("ipsum", "[[1,[1]]]").get_rc_wrapped()
            ),
            to_pl_rc("")
        );

        assert_eq!(
            search(
                "(+lorem +ipsum)",
                TermPostingsListsBuilder::new().with("lorem", "[[1,[1]]]").with("ipsum", "[[1,[1]]]").get_rc_wrapped()
            ),
            to_pl_rc("[[2,[1]]]")
        );

        assert_eq!(
            search(
                "(for ipsum)",
                TermPostingsListsBuilder::new()
                    .with("for", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[2,4,6,8]],[4,[1,3,5,7]]], null, [], [[0,[]],[4,[1,3,5,7]]]")
        );

        assert_eq!(
            search(
                "(lorem lorem)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[8,[1,3,5,7]]], null, null, [[0,[]],[8,[1,3,5,7]]]")
        );

        // SW removal
        assert_eq!(
            search_w_sw_removal(
                "(for ipsum)",
                TermPostingsListsBuilder::new()
                    .with("for", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[2,4,6,8]],[0,[]]], null, []")
        );
    }

    #[test]
    fn test_subtraction_queries() {
        assert_eq!(
            search(
                "lorem -ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("")
        );

        assert_eq!(
            search(
                "(lorem -ipsum)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("null, null, null, [[0,[]],[4,[1,3,5,7]]]")
        );

        assert_eq!(
            search(
                "-lorem -lorem",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("")
        );

        assert_eq!(
            search(
                "+lorem +title -ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], [[2,[1,3]]],        null, [[2,[1,3]]], [[0,[]],[4,[1,3,5,7]]]")
                    .with("title", "[[0,[]],[4,[1,3,5,7]]],        null, [[2,[1,3]]], [[2,[1,3]]], [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "                  null,        null,        null,          [], [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[8,[1,3,5,7]]], null, null, null, null")
        );

        // SW removal
        assert_eq!(
            search(
                "for -ipsum",
                TermPostingsListsBuilder::new()
                    .with("for", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("null, null, null, [[0,[]],[4,[1,3,5,7]]]")
        );

        assert_eq!(
            search_w_sw_removal(
                "for -ipsum",
                TermPostingsListsBuilder::new()
                    .with("for", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("")
        );
    }

    #[test]
    fn test_negation_queries() {
        assert_eq!(
            search(
                "~lorem",
                TermPostingsListsBuilder::new().with("lorem", "null, [[1,[1]]], null, [[1,[1]]]").get_rc_wrapped()
            ),
            to_pl_rc("[], null, [], null, [], [], [], [], [], []")
        );

        assert_eq!(
            search(
                "~lorem ipsum",
                TermPostingsListsBuilder::new().with("lorem", "null, [[1,[1]]], null, [[1,[1]]]").get_rc_wrapped()
            ),
            to_pl_rc("[], null, [], null, [], [], [], [], [], []"),
        );

        assert_eq!(
            search(
                "(~lore ipsum)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,      [[1,[1]]], null, [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]], [[1,[1]]], null, null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[1,[1]]], [[1,[1]]], [], null, [], [], [], [], [], []")
        );

        // SW removal
        assert_eq!(
            search(
                "(~for ipsum)",
                TermPostingsListsBuilder::new()
                    .with("for", "null,      [[1,[1]]], null, [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]], [[1,[1]]], null, null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[1,[1]]], [[1,[1]]], [], null, [], [], [], [], [], []")
        );

        assert_eq!(
            search_w_sw_removal(
                "(~for ipsum)",
                TermPostingsListsBuilder::new()
                    .with("for", "null,      [[1,[1]]], null, [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]], [[1,[1]]], null, null")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[1,[1]]], [[1,[1]]], [], [], [], [], [], [], [], []")
        );
    }

    #[test]
    fn op_order_tests() {
        for query in [
            "title:+lorem ipsum", "+title:lorem ipsum",
            "title:-+lorem ipsum", "-+title:lorem ipsum", "-title:+lorem ipsum",
        ] {
            assert_eq!(
                search(
                    query,
                    TermPostingsListsBuilder::new()
                        .with("lorem", "null,      [[0,[]],[1,[1]]], null, [[1,[1]]]")
                        .with("ipsum", "[[1,[1]]],        [[1,[1]]], null, null")
                        .get_rc_wrapped()
                ),
                to_pl_rc("null, null, null, [[1,[1]]]"),
            );
        }

        for query in [
            "title:-lorem ipsum", "-title:lorem ipsum",
            "title:+-lorem ipsum", "+-title:lorem ipsum", "+title:-lorem ipsum",
        ] {
            assert_eq!(
                search(
                    query,
                    TermPostingsListsBuilder::new()
                        .with("lorem", "null,      [[0,[]],[1,[1]]], null, [[1,[1]]]")
                        .with("ipsum", "[[1,[1]]],        [[1,[1]]], null, [[1,[1]]]")
                        .get_rc_wrapped()
                ),
                to_pl_rc("[[1,[1]]], [[1,[1]]], null, null"),
            );
        }

        for query in ["title:~lorem", "~title:lorem"] {
            assert_eq!(
                search(
                    query,
                    TermPostingsListsBuilder::new()
                        .with("lorem", "null,      [[0,[]],[1,[1]]], null, [[1,[1]]]")
                        .get_rc_wrapped()
                ),
                to_pl_rc("[], [], [], null, [], [], [], [], [], []"),
            );
        }

        for query in [
            "title:~-lorem ipsum", "title:-~lorem ipsum", "~title:-lorem ipsum",
            "-title:~lorem ipsum", "-~title:lorem ipsum", "~-title:lorem ipsum",
        ] {
            assert_eq!(
                search(
                    query,
                    TermPostingsListsBuilder::new()
                        .with("lorem", "null,      [[1,[1]],[1,[1]]], null, [[1,[1]]]")
                        .with("ipsum", "[[1,[1]]],         [[1,[1]]], null, [[1,[1]]]")
                        .get_rc_wrapped()
                ),
                to_pl_rc("null, [[1,[1]]], null, [[1,[1]]]"),
            );
        }

        for query in [
            "title:~+lorem ipsum", "title:+~lorem ipsum", "~title:+lorem ipsum",
            "+title:~lorem ipsum", "+~title:lorem ipsum", "~+title:lorem ipsum",
        ] {
            assert_eq!(
                search(
                    query,
                    TermPostingsListsBuilder::new()
                        .with("lorem", "null,      [[0,[]],[1,[1]]], null, [[1,[1]]]")
                        .with("ipsum", "[[1,[1]]],        [[1,[1]]], null, [[1,[1]]]")
                        .get_rc_wrapped()
                ),
                to_pl_rc("[[1,[1]]], [[1,[1]]], [], null, [], [], [], [], [], []"),
            );
        }
    }
}
