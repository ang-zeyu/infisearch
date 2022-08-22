use std::rc::Rc;

use morsels_common::bitmap;

use crate::postings_list::{self, Field, PlIterator, PostingsList, Doc};
use crate::searcher::query_parser::QueryPart;
use crate::searcher::query_parser::QueryPartType;
use crate::searcher::Searcher;
use crate::utils;

/*
 Processes query operators before the final round in query.rs which ranks the results.
 Postings lists are always still in document id order after being processed here.
 (for efficient processing in AND / NOT / () / Phrase operators)

 Scoring:
 - AND / () operators: scores of expressions within are calculated if necessary and summed
 - Field filters: the filtered expression's score is calculated if necessary
 - NOT: the same score is calculated and assigned to every document in the result set
 - Phrase: scoring is delayed until necessary
 */

fn empty_pl() -> PostingsList {
    PostingsList {
        weight: 1.0,
        include_in_proximity_ranking: true,
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
                    .iter(idx as u8);
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

                            debug_assert!(pl_it.peek_prev().is_some());

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
            while let Some(curr_pl_field) = pl_iters[term_idx].peek_prev().unwrap().fields.get(field_id) {
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
        maybe_partial: bool,
        query_part: &mut QueryPart,
        term_postings_lists: &Vec<Rc<PostingsList>>,
    ) -> Rc<PostingsList> {
        let mut new_pl = empty_pl();
        new_pl.calc_pseudo_idf(self.doc_info.num_docs);
        if query_part.children.is_none() {
            // Empty parentheses
            return Rc::new(new_pl);
        }

        let mut child_postings_lists = self.populate_pls(
            query_part.children.as_mut().unwrap(),
            term_postings_lists,
        );
        if child_postings_lists.is_empty() {
            return Rc::new(new_pl);
        } else if child_postings_lists.len() == 1 {
            return child_postings_lists.pop().unwrap();
        }

        let mut sorted_pl_its: Vec<PlIterator> = child_postings_lists
            .iter()
            .enumerate()
            .map(|(idx, pl_vec)| pl_vec.iter(idx as u8))
            .filter(|pl_it| pl_it.td.is_some())
            .collect();
        let num_pls = sorted_pl_its.len();

        if num_pls == 0
            || (!maybe_partial && num_pls != child_postings_lists.len()) {
            return Rc::new(new_pl);
        }

        loop {
            utils::insertion_sort(&mut sorted_pl_its, |a, b| a.lt(b));

            let min_pl_iter = sorted_pl_its.first().unwrap();
            if let Some(first_td) = min_pl_iter.td {
                let curr_doc_id = first_td.doc_id;
                let mut num_matched_docs = 0;
                for pl_it in sorted_pl_its.iter_mut() {
                    if let Some(td) = pl_it.td {
                        if td.doc_id == curr_doc_id {
                            pl_it.next();

                            debug_assert!(pl_it.peek_prev().is_some());

                            num_matched_docs += 1;
                        }
                    }
                }

                debug_assert!(num_matched_docs > 0);

                // Either:
                // - AND: all previous pls matched
                // - (): always
                if maybe_partial || num_matched_docs == num_pls
                {
                    // Merge the documents with the same doc id

                    // Calculate the new score of the conjunctive expression now (before query.rs)
                    // for preserving the **original** ranking of documents once propagated to the top.
                    let mut acc = Doc { doc_id: curr_doc_id, fields: Vec::new(), score: 0.0 };
                    let mut new_score = 0.0;
                    for pl_it in sorted_pl_its.iter().take(num_matched_docs) {
                        let term_doc = pl_it.peek_prev().unwrap();
                        new_score += if term_doc.score != 0.0 {
                            term_doc.score
                        } else {
                            self.calc_doc_bm25_score(term_doc, curr_doc_id, pl_it.pl)
                        };
                        acc = PostingsList::merge_term_docs(term_doc, &acc);
                    }

                    acc.score = new_score;

                    new_pl.term_docs.push(acc);
                }
            } else {
                break;
            }
        }

        new_pl.calc_pseudo_idf(self.doc_info.num_docs);

        Rc::new(new_pl)
    }

    fn populate_not_postings_list(
        &self,
        query_part: &mut QueryPart,
        term_postings_lists: &Vec<Rc<PostingsList>>,
    ) -> Rc<PostingsList> {
        let mut result_pl = empty_pl();
        result_pl.include_in_proximity_ranking = false;

        let mut not_child_postings_lists =
            self.populate_pls(query_part.children.as_mut().unwrap(), term_postings_lists);

        let mut prev = 0;
        if !not_child_postings_lists.is_empty() {
            debug_assert!(not_child_postings_lists.len() == 1);

            let not_child_postings_list = not_child_postings_lists.remove(0);

            for td in not_child_postings_list.term_docs.iter() {
                for doc_id in prev..td.doc_id {
                    if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                        result_pl.term_docs.push(Doc { doc_id, fields: Vec::new(), score: 0.0 });
                    }
                }
                prev = td.doc_id + 1;
            }
        }

        for doc_id in prev..self.doc_info.doc_length_factors_len {
            if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                result_pl.term_docs.push(Doc { doc_id, fields: Vec::new(), score: 0.0 });
            }
        }

        result_pl.calc_pseudo_idf(self.doc_info.num_docs);

        // Same score for every result. Score resulting from field tf is taken as 1.
        let score = result_pl.idf as f32 * result_pl.weight;
        for term_doc in result_pl.term_docs.iter_mut() {
            term_doc.score = score;
        }

        Rc::new(result_pl)
    }

    fn filter_field_postings_list(&self, field_name: &str, pl: &mut Rc<PostingsList>) {
        let field_id_and_info = self
            .searcher_config
            .field_infos
            .iter()
            .enumerate()
            .find(|(_id, field_info)| field_info.name == field_name);

        if let Some((field_id, _field_info)) = field_id_and_info {
            let mut new_pl = PostingsList {
                weight: pl.weight,
                include_in_proximity_ranking: pl.include_in_proximity_ranking,
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
                        self.calc_doc_bm25_score(term_doc, term_doc.doc_id, pl)
                    };
                    new_pl.term_docs.push(Doc { doc_id: term_doc.doc_id, fields, score })
                }
            }

            *pl = Rc::new(new_pl);
        }
    }

    pub fn populate_pls(
        &self,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: &Vec<Rc<PostingsList>>,
    ) -> Vec<Rc<PostingsList>> {
        let mut result: Vec<Rc<PostingsList>> = Vec::new();

        for query_part in query_parts {
            let mut pl_opt: Option<Rc<PostingsList>> = None;
            match query_part.part_type {
                QueryPartType::Term => {
                    if let Some(terms) = &query_part.terms {
                        if let Some(term) = terms.first() {
                            if let Some(term_pl) = postings_list::get_postings_list_rc(term, term_postings_lists) {
                                pl_opt = Some(Rc::clone(term_pl));
                            }
                        } /* else {
                            spelling correct / stop word removed, ignore
                        } */
                    } /* else {
                        spelling correct / stop word removed, ignore
                    } */
                }
                QueryPartType::Phrase => {
                    if query_part.terms.as_ref().unwrap().len() == 1 {
                        if let Some(term_pl) = postings_list::get_postings_list_rc(
                            query_part.terms.as_ref().unwrap().first().unwrap(),
                            term_postings_lists,
                        ) {
                            pl_opt = Some(Rc::clone(term_pl));
                        }
                    } else {
                        pl_opt = Some(self.populate_phrasal_postings_lists(query_part, term_postings_lists));
                    }
                }
                QueryPartType::And => {
                    pl_opt = Some(self.populate_conjunctive_postings_lists(false, query_part, term_postings_lists));
                }
                QueryPartType::Not => {
                    pl_opt = Some(self.populate_not_postings_list(query_part, term_postings_lists));
                }
                QueryPartType::Bracket => {
                    pl_opt = Some(self.populate_conjunctive_postings_lists(true, query_part, term_postings_lists));
                }
            }

            if let Some(mut pl) = pl_opt {
                if let Some(field_name) = &query_part.field_name {
                    self.filter_field_postings_list(field_name, &mut pl);
                }

                result.push(pl);
            } else {
                result.push(Rc::new(empty_pl()))
            }
        }

        result
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

    fn search(query: &str, term_postings_lists: Vec<Rc<PostingsList>>) -> Vec<Rc<PostingsList>> {
        let mut parsed = query_parser_test::parse(query);
        searcher_test::create_searcher(10, 3).populate_pls(
            &mut parsed,
            &term_postings_lists,
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
            vec![to_pl_rc("[[2,[12,31]],[0,[]],[0,[]]]")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[0,[]],[3,[1,12,31]]]")
                    .with("ipsum", "[[0,[]],[0,[]],[3,[11,13,32]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[0,[]],[0,[]],[2,[12,31]]]")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null, null, [[0,[]],[0,[]],[3,[1,12,31]]]")
                    .with("ipsum", "null, null, [[0,[]],[0,[]],[3,[11,13,32]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("null, null, [[0,[]],[0,[]],[2,[12,31]]]")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[4,[1,3,5,7]],[0,[]],[0,[]]]")]
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
            vec![to_pl_rc("")]
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
            vec![to_pl_rc("")]
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
            vec![to_pl_rc("")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null, null, [[0,[]],[3,[1,12,31]],         [0,[]]]")
                    .with("ipsum", "null, null, [[0,[]],[0,[]],[3,[11,13,32]]]        ")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("")]
        );
    }

    #[test]
    fn test_and_queries() {
        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "")
                    .with("ipsum", "")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("")]
        );

        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]]]")
                    .with("ipsum", "")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("")]
        );

        // Different fields still match
        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[2,[1]]]")]
        );

        // Test position, field merging behaviour
        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[1,[2]]]")
                    .with("ipsum", "[[1,[10]],[1,[1]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[1,[10]],[2,[1,2]]]")]
        );

        // Multiple docs
        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[3,[1,2,8]]], [[0,[]],[1,[1]]], [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]           , null            , [[3,[1,5,9]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[1,[1]],[3,[1,2,8]]], null, [[4,[1,5,9]]]")]
        );
    }

    #[test]
    fn test_and_queries_negative() {
        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,     [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("")]
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
            vec![
                to_pl_rc("[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]"),
                to_pl_rc("[[4,[2,4,6,8]],[0,[]]], null, []  , null"),
            ]
        );

        assert_eq!(
            search(
                "lorem lorem",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            vec![
                to_pl_rc("[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]"),
                to_pl_rc("[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]"),
            ]
        );
    }

    #[test]
    fn test_parentheses_queries() {
        assert_eq!(
            search(
                "(lorem AND ipsum)",
                TermPostingsListsBuilder::new().with("lorem", "[[1,[1]]]").with("ipsum", "[[1,[1]]]").get_rc_wrapped()
            ),
            vec![to_pl_rc("[[2,[1]]]")]
        );

        assert_eq!(
            search(
                "(lorem ipsum)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[4,[2,4,6,8]],[4,[1,3,5,7]]], null, [], [[0,[]],[4,[1,3,5,7]]]")]
        );

        assert_eq!(
            search(
                "(lorem lorem)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[0,[]],[8,[1,3,5,7]]], null, null, [[0,[]],[8,[1,3,5,7]]]")]
        );
    }

    #[test]
    fn test_not_queries() {
        assert_eq!(
            search(
                "NOT lorem",
                TermPostingsListsBuilder::new().with("lorem", "null, [[1,[1]]], null, [[1,[1]]]").get_rc_wrapped()
            ),
            vec![to_pl_rc("[], null, [], null, [], [], [], [], [], []")]
        );

        assert_eq!(
            search(
                "NOT lorem ipsum",
                TermPostingsListsBuilder::new().with("lorem", "null, [[1,[1]]], null, [[1,[1]]]").get_rc_wrapped()
            ),
            vec![
                to_pl_rc("[], null, [], null, [], [], [], [], [], []"),
                to_pl_rc(""),
            ]
        );

        assert_eq!(
            search(
                "(NOT lorem ipsum)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,      [[1,[1]]], null, [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]], [[1,[1]]], null, null")
                    .get_rc_wrapped()
            ),
            vec![to_pl_rc("[[1,[1]]], [[1,[1]]], [], null, [], [], [], [], [], []")]
        );
    }
}
