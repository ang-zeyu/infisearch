mod bm25;
mod proximity_ranking;

use std::cmp::Ordering;
use std::rc::Rc;

use binary_heap_plus::BinaryHeap;
use infisearch_common::bitmap;
use infisearch_common::metadata::EnumMax;
use infisearch_common::utils::push;

use crate::doc_info::DocInfo;
use crate::postings_list::{self, Field, PlIterator, PostingsList, Doc, PlAndInfo};
use crate::searcher::query_parser::QueryPart;
use crate::searcher::query_parser::QueryPartType;
use crate::searcher::Searcher;

use super::query::{DocResult, DocResultComparator};


fn empty_pl() -> PostingsList {
    PostingsList {
        term_docs: Vec::new(),
        idf: 0.0,
        term: None,
        term_info: None,
    }
}

impl Searcher {
    fn populate_conjunctive_postings_lists(
        &self,
        is_bracket: bool,
        is_phrase: bool,
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
            .map(|pl_and_info| pl_and_info.pl.iter(
                pl_and_info.weight,
                pl_and_info.include_in_proximity_ranking,
                pl_and_info.is_mandatory,
                pl_and_info.is_subtracted,
                pl_and_info.is_inverted,
            ))
            .collect();

        let num_mandatory_pls = sorted_pl_its.iter()
            .filter(|pl_it| pl_it.is_mandatory)
            .count();

        // ------------------------------------------
        // Query term proximity ranking
        const MAX_WINDOW_LEN: u32 = 200;
        const PROXIMITY_BASE_SCALING: f32 = 2.5;
        const PROXIMITY_PER_TERM_SCALING: f32 = 0.5;

        let max_window_len = if is_phrase {
            if num_mandatory_pls > 0 {
                (num_mandatory_pls - 1) as u32
            } else {
                0
            }
        } else {
            MAX_WINDOW_LEN
        };

        let total_pls = child_postings_lists.iter().filter(|pl| !pl.is_subtracted).count() as f32;

        let total_proximity_ranking_pls = child_postings_lists.iter()
            .filter(|pl_and_info| pl_and_info.include_in_proximity_ranking && (!is_phrase || pl_and_info.is_mandatory))
            .count();
        let min_proximity_ranking_pls = if is_phrase {
            total_proximity_ranking_pls
        } else {
            (total_proximity_ranking_pls as f32 / 2.0).ceil() as usize
        }.max(2);

        let proximity_scaling = PROXIMITY_BASE_SCALING
            + (total_proximity_ranking_pls as f32 * PROXIMITY_PER_TERM_SCALING);

        // For proximity_ranking::rank, to minimize allocations
        let mut positions = Vec::with_capacity(
            total_proximity_ranking_pls * self.searcher_config.num_scored_fields,
        );

        let do_run_proximity = self.searcher_config.searcher_options.use_query_term_proximity || is_phrase;

        #[cfg(feature="perf")]
        web_sys::console::log_1(
            &format!("total_proximity_ranking_pls {} min_proximity_ranking_pls {}",
            total_proximity_ranking_pls, min_proximity_ranking_pls,
        ).into());

        // ------------------------------------------

        let do_accumulate = is_bracket || (is_phrase && total_proximity_ranking_pls == 1);

        // Heuristic, exact size can't be known without processing
        new_pl.term_docs.reserve_exact(
            child_postings_lists.iter().map(|pl| pl.pl.term_docs.len()).max().unwrap_or(128),
        );

        loop {
            let doc_id = if num_mandatory_pls > 0 {
                // Find the largest mandatory id for forwarding other postings lists
                let id = unsafe {
                    sorted_pl_its
                        .iter()
                        .filter_map(|pl_it| if pl_it.is_mandatory {
                            Some(pl_it.td.map(|td| td.doc_id).unwrap_or(std::u32::MAX))
                        } else {
                            None
                        })
                        .max()
                        .unwrap_unchecked() // guaranteed by num_mandatory_pls > 0
                };

                if id < std::u32::MAX {
                    id
                } else {
                    break;
                }
            } else if let Some(first_id) = sorted_pl_its
                .iter()
                .filter_map(|pl_it| pl_it.td.map(|doc| doc.doc_id))
                .min()
            {
                first_id
            } else {
                break;
            };

            let mut score = 0.0;
            let mut num_pls_matched = 0;
            let mut num_mandatory_pls_matched = 0;
            let mut num_proximity_ranking_pls = 0;
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

                            num_pls_matched += 1;

                            if pl_it.is_mandatory {
                                num_mandatory_pls_matched += 1;
                            }

                            if pl_it.include_in_proximity_ranking
                                && (!is_phrase || pl_it.is_mandatory) {
                                num_proximity_ranking_pls += 1;
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
            if do_run_proximity && num_proximity_ranking_pls >= min_proximity_ranking_pls {
                let phrase_query_res = proximity_ranking::rank(
                    is_phrase,
                    max_window_len,
                    self.searcher_config.num_scored_fields,
                    &sorted_pl_its,
                    proximity_scaling,
                    &mut positions,
                    doc_id,
                    total_proximity_ranking_pls,
                    min_proximity_ranking_pls,
                    &mut positional_scaling_factor,
                );

                if is_phrase {
                    if let Some(mut doc) = phrase_query_res {
                        doc.score = score * positional_scaling_factor;
                        new_pl.term_docs.push(doc);
                    }
                    continue;
                }
            }
            // ------------------------------------------

            if !is_subtracted && !(num_mandatory_pls > 0 && num_mandatory_pls_matched < num_mandatory_pls) {
                let conjunctive_scaling_factor = num_pls_matched as f32 / total_pls;
                acc.score = score
                    * positional_scaling_factor
                    * conjunctive_scaling_factor
                    * conjunctive_scaling_factor;
                new_pl.term_docs.push(acc);
            }
        }

        new_pl.calc_pseudo_idf(self.doc_info.num_docs);

        new_pl
    }

    fn invert_postings_list(&self, pl: Rc<PostingsList>, weight: f32) -> Rc<PostingsList> {
        let mut result_pl = PostingsList {
            term_docs: Vec::with_capacity(self.doc_info.doc_length_factors_len as usize - pl.term_docs.len()),
            idf: 0.0,
            term: None,
            term_info: None,
        };

        let mut prev = 0;
        for td in pl.term_docs.iter() {
            for doc_id in prev..td.doc_id {
                if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                    push::push_wo_grow(
                        &mut result_pl.term_docs,
                        Doc { doc_id, fields: Vec::new(), score: 0.0 },
                    );
                }
            }
            prev = td.doc_id + 1;
        }

        for doc_id in prev..self.doc_info.doc_length_factors_len {
            if !bitmap::check(&self.invalidation_vector, doc_id as usize) {
                push::push_wo_grow(
                    &mut result_pl.term_docs,
                    Doc { doc_id, fields: Vec::new(), score: 0.0 },
                );
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
                term_docs: Vec::with_capacity(pl.term_docs.len()),
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
                    push::push_wo_grow(&mut new_pl.term_docs, Doc { doc_id: term_doc.doc_id, fields, score });
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
        let mut result: Vec<PlAndInfo> = Vec::with_capacity(query_parts.len());

        for query_part in query_parts {
            let mut pl_opt: Option<Rc<PostingsList>> = None;
            let weight = weight * query_part.weight;

            if let Some(children) = &mut query_part.children {
                debug_assert!(
                    query_part.term.is_none()
                    && (
                        matches!(query_part.part_type, QueryPartType::Bracket)
                        || matches!(query_part.part_type, QueryPartType::Phrase)
                    )
                );

                let is_phrase = matches!(query_part.part_type, QueryPartType::Phrase);
                pl_opt = Some(Rc::new(self.populate_conjunctive_postings_lists(
                    !is_phrase, is_phrase, children, term_postings_lists, weight,
                )));
            } else if let Some(term) = &query_part.term {
                debug_assert!(
                    query_part.children.is_none()
                    && matches!(query_part.part_type, QueryPartType::Term)
                );

                if let Some(term_pl) = postings_list::get_postings_list_rc(term, term_postings_lists) {
                    pl_opt = Some(Rc::clone(term_pl));
                }
            }

            let mut pl = pl_opt.unwrap_or(Rc::new(empty_pl()));

            if let Some(field_name) = &query_part.field_name {
                self.filter_field_postings_list(field_name, &mut pl, weight);
            }

            // Negation after field filter. If before, it would just return an empty list.
            if query_part.is_inverted {
                pl = self.invert_postings_list(pl, weight);
            }

            push::push_wo_grow(&mut result, PlAndInfo {
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
        enum_filters: Vec<(usize, [bool; EnumMax::MAX as usize])>,
        i64_filters: Vec<(usize, Option<i64>, Option<i64>)>,
        num_sort: Option<usize>,
        reverse_sort: bool,
    ) -> BinaryHeap<DocResult, Box<DocResultComparator>> {
        let root_pl = self.populate_conjunctive_postings_lists(
            false, false, query_parts, term_postings_lists, 1.0,
        );

        let mut doc_results = Vec::with_capacity(root_pl.term_docs.len());
        for td in root_pl.term_docs {
            let passes_enum_filters = enum_filters
                .iter()
                .all(|(enum_id, ev_ids)| {
                    let ev_id = self.doc_info.get_enum_val(td.doc_id as usize, *enum_id) as usize;
                    debug_assert!(ev_id < ev_ids.len());
                    unsafe { *ev_ids.get_unchecked(ev_id) }
                });

            let passes_i64_filters = i64_filters
                .iter()
                .all(|(id, gte, lte)| {
                    let v = self.doc_info.get_num_val(td.doc_id as usize, *id);

                    let satisfies_gte = if let Some(lower_bound) = gte {
                        v >= *lower_bound
                    } else {
                        true
                    };

                    let satisfies_lte = if let Some(upper_bound) = lte {
                        v <= *upper_bound
                    } else {
                        true
                    };

                    satisfies_gte && satisfies_lte
                });

            if passes_enum_filters && passes_i64_filters {
                push::push_wo_grow(&mut doc_results, DocResult { doc_id: td.doc_id, score: td.score });
            }
        }

        let doc_info_pointer = &self.doc_info as *const DocInfo;
        BinaryHeap::from_vec_cmp(doc_results, Box::new(move |a: &DocResult, b: &DocResult| {
            let mut cmp = if let Some(num_sort) = num_sort {
                let doc_info = unsafe { &*doc_info_pointer };
                let value_a = doc_info.get_num_val(a.doc_id as usize, num_sort);
                let value_b = doc_info.get_num_val(b.doc_id as usize, num_sort);
                if reverse_sort {
                    value_b.cmp(&value_a)
                } else {
                    value_a.cmp(&value_b)
                }
            } else {
                Ordering::Equal
            };

            if let Ordering::Equal = cmp {
                cmp = unsafe { a.score.partial_cmp(&b.score).unwrap_unchecked() };
            }

            cmp
        }))
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
                "\" \"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[3,[1,12,31]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        assert_eq!(
            search(
                "\"lorem\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[3,[1,12,31]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[3,[1,12,31]]]"),
        );

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

        // Same word
        assert_eq!(
            search(
                "\"lorem lorem\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,2,3,4]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[2,[1,3]],[0,[]]], null, null, null"),
        );

        assert_eq!(
            search(
                "\"lorem lorem lorem\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,2,3,4]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[1,[1]],[0,[]],[0,[]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum lorem\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[2,[1,3]]]")
                    .with("ipsum", "[[0,[]],[1,[2]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[1,[1]],[0,[]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[2,[1,3]]]")
                    .with("ipsum", "[[1,[4]],[1,[2]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[0,[]],[1,[1]],[0,[]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]],[1,[3]]]")
                    .with("ipsum", "[[1,[4]],[1,[2]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[1,[1]],[0,[]],[0,[]]]"),
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
            to_pl_rc("[[0,[]],[2,[12,31]],[0,[]]]"),
        );

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null, null, [[0,[]],[3,[1,12,31]],         [0,[]]]")
                    .with("ipsum", "null, null, [[0,[]],[0,[]],[3,[11,13,32]]]        ")
                    .get_rc_wrapped()
            ),
            to_pl_rc("null, null, [[0,[]],[2,[12,31]],[0,[]]]"),
        );

        // SW removal, spelling correction
        assert_eq!(
            search(
                "\"lore ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[4,[1,3,5,7]],[0,[]],[0,[]]]"),
        );

        for q in ["\"nonexistentterm lore ipsum\"", "\"lore nonexistentterm ipsum\""] {
            assert_eq!(
                search(
                    q,
                    TermPostingsListsBuilder::new()
                        .with("lorem", "[[4,[1,3,5,7]]]")
                        .with("ipsum", "[[4,[2,4,6,8]]]")
                        .get_rc_wrapped()
                ),
                to_pl_rc("[[4,[1,3,5,7]],[0,[]],[0,[]]]"),
            );
        }

        for q in ["\"for lore ipsum\"", "\"lore for ipsum\""] {
            assert_eq!(
                search_w_sw_removal(
                    q,
                    TermPostingsListsBuilder::new()
                        .with("lorem", "[[4,[1,3,5,7]]]")
                        .with("ipsum", "[[4,[2,4,6,8]]]")
                        .with("for", "[[4,[1,3,5,7]]]")
                        .get_rc_wrapped()
                ),
                to_pl_rc("[[4,[1,3,5,7]],[0,[]],[0,[]]]"),
            );
        }
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

        assert_eq!(
            search(
                "\"lorem ipsum\"",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,                           null, [[0,[]],[3,[1,12,31]], [0,[]]]")
                    .with("ipsum", "null, [[0,[]],[0,[]],[3,[11,13,32]]]")
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

        // Different documents
        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "null,      [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]], [[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("null, [[2,[1]]]"),
        );

        assert_eq!(
            search(
                "+lorem +ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]]], [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[2,[1]]]"),
        );

        assert_eq!(
            search(
                "+lorem ipsum",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[1,[1]]], null,     [[1,[1]]]")
                    .with("ipsum", "[[1,[1]]], [[1,[1]]]")
                    .get_rc_wrapped()
            ),
            to_pl_rc("[[2,[1]]], null,     [[1,[1]]]"),
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
                "",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

        assert_eq!(
            search(
                " ",
                TermPostingsListsBuilder::new()
                    .with("lorem", "[[0,[]],[4,[1,3,5,7]]], null, null, [[0,[]],[4,[1,3,5,7]]]")
                    .with("ipsum", "[[4,[2,4,6,8]],[0,[]]], null, [],   null")
                    .get_rc_wrapped()
            ),
            to_pl_rc(""),
        );

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
