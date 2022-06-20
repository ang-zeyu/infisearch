use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::rc::Rc;

use morsels_common::bitmap;

use crate::postings_list::DocField;
use crate::postings_list::PlIterator;
use crate::postings_list::PostingsList;
use crate::postings_list::TermDoc;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::query_parser::QueryPartType;
use crate::searcher::Searcher;

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
        term_postings_lists: &HashMap<String, Rc<PostingsList>>,
    ) -> Rc<PostingsList> {
        let mut encountered_empty_pl = false;
        let pl_iterators: Vec<Rc<RefCell<PlIterator>>> = query_part
            .terms
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(idx, term)| {
                let pl_iterator = term_postings_lists.get(term).unwrap().get_it(idx as u8);
                if pl_iterator.td.is_none() {
                    encountered_empty_pl = true;
                }
                Rc::new(RefCell::new(pl_iterator))
            })
            .collect();

        let mut result_pl = empty_pl();

        if encountered_empty_pl {
            return Rc::new(result_pl);
        }
        
        let mut iterator_heap: BinaryHeap<Reverse<Rc<RefCell<PlIterator>>>> = pl_iterators
            .iter()
            .map(|pl_it| Reverse(Rc::clone(pl_it)))
            .collect();
        let num_pls = iterator_heap.len();

        let mut curr_doc_id = self.doc_info.get_non_existent_id();
        let mut curr_num_docs = 0;
        while !iterator_heap.is_empty() {
            let min_pl_iterator_rc = iterator_heap.pop().unwrap();
            let mut min_pl_iterator = min_pl_iterator_rc.0.borrow_mut();

            // Do an "AND" query first
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

                // Now do the phrase query on curr_doc_id

                let mut td = TermDoc { doc_id: curr_doc_id, fields: Vec::new(), score: 0.0 };
                let mut has_match = false;

                let termdocs: Vec<&TermDoc> = pl_iterators
                    .iter()
                    .map(|pl_it| pl_it.borrow().peek_prev().unwrap())
                    .collect();

                for field_id in 0..self.searcher_config.num_scored_fields as usize {
                    let mut result_doc_field = DocField { field_tf: 0.0, field_positions: Vec::new() };

                    let mut term_field_position_idxes = vec![0; num_pls];
                    let mut curr_pos: u32 = 0;
                    let mut term_idx = 0;

                    // Go through the terms in this field, controlled by term_idx modifications below
                    while let Some(curr_pl_field) = termdocs[term_idx].fields.get(field_id) {
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

                curr_doc_id = self.doc_info.get_non_existent_id();
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
        term_postings_lists: &HashMap<String, Rc<PostingsList>>,
    ) -> Rc<PostingsList> {
        let mut pl_vecs = self.populate_postings_lists(query_part.children.as_mut().unwrap(), term_postings_lists);
        if pl_vecs.len() == 1 {
            return pl_vecs.remove(0);
        }

        let mut doc_heap: BinaryHeap<Reverse<PlIterator>> = pl_vecs
            .iter()
            .enumerate()
            .map(|(idx, pl_vec)| Reverse(pl_vec.get_it(idx as u8)))
            .filter(|pl_it| pl_it.0.td.is_some())
            .collect();
        let num_pls = doc_heap.len();

        let mut result_pl = empty_pl();

        if num_pls != pl_vecs.len() {
            return Rc::new(result_pl);
        }

        let mut curr_doc_id: u32 = self.doc_info.get_non_existent_id();
        let mut curr_num_docs = 0;
        while !doc_heap.is_empty() {
            let mut min_pl_iterator = doc_heap.pop().unwrap();

            if min_pl_iterator.0.td.unwrap().doc_id == curr_doc_id {
                if min_pl_iterator.0.next().is_some() {
                    doc_heap.push(min_pl_iterator);
                }

                curr_num_docs += 1;

                if curr_num_docs == num_pls {
                    let mut acc = TermDoc { doc_id: curr_doc_id, fields: Vec::new(), score: 0.0 };
                    for td in doc_heap.iter().map(|pl_it| pl_it.0.peek_prev().unwrap()) {
                        acc = PostingsList::merge_term_docs(td, &acc);
                    }

                    acc.score = self.sum_scores(doc_heap.iter(), curr_doc_id);

                    result_pl.term_docs.push(acc);

                    curr_doc_id = self.doc_info.get_non_existent_id();
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
        term_postings_lists: &HashMap<String, Rc<PostingsList>>,
    ) -> Rc<PostingsList> {
        let mut result_pl = empty_pl();
        result_pl.include_in_proximity_ranking = false;

        let mut not_child_postings_lists =
            self.populate_postings_lists(query_part.children.as_mut().unwrap(), term_postings_lists);

        let mut prev = 0;
        if !not_child_postings_lists.is_empty() {
            debug_assert!(not_child_postings_lists.len() == 1);

            let not_child_postings_list = not_child_postings_lists.remove(0);

            for td in not_child_postings_list.term_docs.iter() {
                for doc_id in prev..td.doc_id {
                    if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                        result_pl.term_docs.push(TermDoc { doc_id, fields: Vec::new(), score: 0.0 });
                    }
                }
                prev = td.doc_id + 1;
            }
        }

        for doc_id in prev..self.doc_info.doc_length_factors_len {
            if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                result_pl.term_docs.push(TermDoc { doc_id, fields: Vec::new(), score: 0.0 });
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

    fn populate_bracket_postings_list(
        &self,
        query_part: &mut QueryPart,
        term_postings_lists: &HashMap<String, Rc<PostingsList>>,
    ) -> Rc<PostingsList> {
        let mut new_pl = empty_pl();
        if query_part.children.is_none() {
            return Rc::new(new_pl);
        }

        let mut child_postings_lists = self.populate_postings_lists(
            query_part.children.as_mut().unwrap(),
            term_postings_lists,
        );

        if child_postings_lists.is_empty() {
            return Rc::new(new_pl);
        } else if child_postings_lists.len() == 1 {
            return child_postings_lists.pop().unwrap();
        }

        let mut doc_heap: BinaryHeap<Reverse<PlIterator>> = child_postings_lists
            .iter()
            .enumerate()
            .map(|(idx, pl_vec)| Reverse(pl_vec.get_it(idx as u8)))
            .filter(|pl_it| pl_it.0.td.is_some())
            .collect();
        let num_pls = doc_heap.len();


        if num_pls == 0 {
            new_pl.calc_pseudo_idf(self.doc_info.num_docs);
            return Rc::new(new_pl);
        }

        let mut curr_pl_iterators: Vec<Reverse<PlIterator>> = Vec::with_capacity(num_pls);
        while !doc_heap.is_empty() {
            let curr_pl_it = doc_heap.pop().unwrap();
            let doc_id = curr_pl_it.0.td.unwrap().doc_id;

            curr_pl_iterators.push(curr_pl_it);

            while !doc_heap.is_empty() && doc_heap.peek().unwrap().0.td.unwrap().doc_id == doc_id {
                curr_pl_iterators.push(doc_heap.pop().unwrap());
            }

            let mut merged_term_docs = if curr_pl_iterators.len() == 1 {
                curr_pl_iterators[0].0.td.unwrap().to_owned()
            } else {
                curr_pl_iterators.iter().fold(TermDoc { doc_id, fields: Vec::new(), score: 0.0 }, |acc, next| {
                    PostingsList::merge_term_docs(&acc, next.0.td.unwrap())
                })
            };

            merged_term_docs.score = self.sum_scores(curr_pl_iterators.iter(), doc_id);

            new_pl.term_docs.push(merged_term_docs);

            for mut pl_it in curr_pl_iterators.drain(..) {
                if pl_it.0.next().is_some() {
                    doc_heap.push(pl_it);
                }
            }
        }

        new_pl.calc_pseudo_idf(self.doc_info.num_docs);

        Rc::new(new_pl)
    }

    // ---------------------------------------
    // Calculate the new score of the disjunctive expression now (before query.rs)
    // for preserving the **original** ranking of documents once propagated to the top.
    fn sum_scores<'a, T>(&self, curr_pl_iterators: T, doc_id: u32) -> f32
    where T: Iterator<Item = &'a Reverse<PlIterator<'a>>>
    {
        let mut new_score = 0.0;
        for pl_it in curr_pl_iterators {
            let score = pl_it.0.td.unwrap().score;
            new_score += if score != 0.0 {
                score
            } else {
                self.calc_doc_bm25_score(pl_it.0.td.unwrap(), doc_id, pl_it.0.pl)
            };
        }
        new_score
    }
    // ---------------------------------------

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

            let fields_before = vec![DocField::default(); field_id];
            for term_doc in &pl.term_docs {
                if let Some(doc_field) = term_doc.fields.get(field_id) {
                    if doc_field.field_tf == 0.0 {
                        continue;
                    }

                    let mut fields: Vec<DocField> = fields_before.clone();
                    fields.push(doc_field.clone()); // TODO reduce potential allocations?

                    let score = if term_doc.score != 0.0 {
                        term_doc.score
                    } else {
                        self.calc_doc_bm25_score(term_doc, term_doc.doc_id, pl)
                    };
                    new_pl.term_docs.push(TermDoc { doc_id: term_doc.doc_id, fields, score })
                }
            }

            *pl = Rc::new(new_pl);
        }
    }

    fn populate_postings_lists(
        &self,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: &HashMap<String, Rc<PostingsList>>,
    ) -> Vec<Rc<PostingsList>> {
        let mut result: Vec<Rc<PostingsList>> = Vec::new();

        for query_part in query_parts {
            let mut pl_opt: Option<Rc<PostingsList>> = None;
            match query_part.part_type {
                QueryPartType::Term => {
                    if let Some(term) = query_part.terms.as_ref().unwrap().first() {
                        if let Some(term_pl) = term_postings_lists.get(term) {
                            pl_opt = Some(Rc::clone(term_pl));
                        }
                    } /* else {
                        spelling correct / stop word removed, ignore
                    } */
                }
                QueryPartType::Phrase => {
                    if query_part.terms.as_ref().unwrap().len() == 1 {
                        if let Some(term_pl) = term_postings_lists.get(query_part.terms.as_ref().unwrap().first().unwrap()) {
                            pl_opt = Some(Rc::clone(term_pl));
                        }
                    } else {
                        pl_opt = Some(self.populate_phrasal_postings_lists(query_part, term_postings_lists));
                    }
                }
                QueryPartType::And => {
                    pl_opt = Some(self.populate_and_postings_lists(query_part, term_postings_lists));
                }
                QueryPartType::Not => {
                    pl_opt = Some(self.populate_not_postings_list(query_part, term_postings_lists));
                }
                QueryPartType::Bracket => {
                    pl_opt = Some(self.populate_bracket_postings_list(query_part, term_postings_lists));
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

    pub fn process(
        &self,
        query_parts: &mut Vec<QueryPart>,
        term_postings_lists: HashMap<String, PostingsList>,
    ) -> Vec<Rc<PostingsList>> {
        let term_rc_postings_lists: HashMap<String, Rc<PostingsList>> =
            term_postings_lists.into_iter().map(|(term, pl)| (term, Rc::new(pl))).collect();

        self.populate_postings_lists(query_parts, &term_rc_postings_lists)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::rc::Rc;

    use pretty_assertions::assert_eq;

    use crate::postings_list::test::{to_pl, to_pl_rc};
    use crate::postings_list::PostingsList;
    use crate::searcher::query_parser::test as query_parser_test;
    use crate::searcher::test as searcher_test;

    struct TermPostingsListsBuilder(HashMap<String, PostingsList>);

    impl TermPostingsListsBuilder {
        fn new() -> Self {
            TermPostingsListsBuilder(HashMap::default())
        }

        fn with(mut self, term: &str, pl_str: &str) -> Self {
            self.0.insert(term.to_owned(), to_pl(pl_str));
            self
        }
    }

    fn search(query: &str, term_postings_lists: HashMap<String, PostingsList>) -> Vec<Rc<PostingsList>> {
        let mut parsed = query_parser_test::parse(query);
        searcher_test::create_searcher(10, 3).process(&mut parsed, term_postings_lists)
    }

    #[test]
    fn test_phrasal_queries() {
        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[3,[1,12,31]]]")
                    .with("ipsum", "[[3,[11,13,32]]]")
                    .0
            ),
            vec![to_pl_rc("[[2,[12,31]],[0,[]],[0,[]]]")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[0,[]],[3,[1,12,31]]]")
                    .with("ipsum", "[[0,[]],[0,[]],[3,[11,13,32]]]")
                    .0
            ),
            vec![to_pl_rc("[[0,[]],[0,[]],[2,[12,31]]]")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null, null, [[0,[]],[0,[]],[3,[1,12,31]]]")
                    .with("ipsum", "null, null, [[0,[]],[0,[]],[3,[11,13,32]]]")
                    .0
            ),
            vec![to_pl_rc("null, null, [[0,[]],[0,[]],[2,[12,31]]]")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]]]")
                    .0
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
                    .0
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
                    .0
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
                    .0
            ),
            vec![to_pl_rc("")]
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null, null, [[0,[]],[3,[1,12,31]],         [0,[]]]")
                    .with("ipsum", "null, null, [[0,[]],[0,[]],[3,[11,13,32]]]        ")
                    .0
            ),
            vec![to_pl_rc("")]
        );
    }

    #[test]
    fn test_and_queries() {
        // Different fields still match
        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new().with("lorem", "[[1,[1]]]").with("ipsum", "[[1,[1]]]").0
            ),
            vec![to_pl_rc("[]")]
        );

        // Different fields still match
        assert_eq!(
            search(
                "lorem AND ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .0
            ),
            vec![to_pl_rc("[]")]
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
                    .0
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
                    .0
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
                    .0
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
                TermPostingsListsBuilder::new().with("lorem", "[[1,[1]]]").with("ipsum", "[[1,[1]]]").0
            ),
            vec![to_pl_rc("[]")]
        );

        assert_eq!(
            search(
                "(lorem ipsum)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .0
            ),
            vec![to_pl_rc("[[4,[2,4,6,8]],[4,[1,3,5,7]]], null, [], [[0,[]],[4,[1,3,5,7]]]")]
        );

        assert_eq!(
            search(
                "(lorem lorem)",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .0
            ),
            vec![to_pl_rc("[[0,[]],[8,[1,3,5,7]]], null, null, [[0,[]],[8,[1,3,5,7]]]")]
        );
    }

    #[test]
    fn test_not_queries() {
        assert_eq!(
            search(
                "NOT lorem",
                TermPostingsListsBuilder::new().with("lorem", "null, [[1,[1]]], null, [[1,[1]]]").0
            ),
            vec![to_pl_rc("[], null, [], null, [], [], [], [], [], []")]
        );

        assert_eq!(
            search(
                "NOT lorem ipsum",
                TermPostingsListsBuilder::new().with("lorem", "null, [[1,[1]]], null, [[1,[1]]]").0
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
                    .0
            ),
            vec![to_pl_rc("[[1,[1]]], [[1,[1]]], [], null, [], [], [], [], [], []")]
        );
    }
}
