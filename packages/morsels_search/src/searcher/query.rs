use std::cmp::Ordering;
use std::collections::BinaryHeap;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::searcher::query_parser::{self, QueryPart};
use crate::searcher::Searcher;

#[derive(Clone)]
pub struct DocResult {
    pub doc_id: u32,
    pub score: f32,
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
    pub fn get_next_n(&mut self, n: usize) -> Vec<u32> {
        let mut doc_ids: Vec<u32> = Vec::with_capacity(n);
        while !self.result_heap.is_empty()
            && doc_ids.len() < n
            && (self.result_limit.is_none() || self.results_retrieved < self.result_limit.unwrap())
        {
            doc_ids.push(self.result_heap.pop().unwrap().doc_id);
            self.results_retrieved += 1;
        }

        doc_ids
    }

    pub fn get_query_parts(&self) -> String {
        QueryPart::serialize_parts(&self.query_parts)
    }

    pub fn get_searched_terms(&self) -> String {
        let mut output = "[".to_owned();
        let wrapped: Vec<String> = self.searched_terms.iter().map(|term_group| {
            query_parser::serialize_string_vec(term_group)
        }).collect();
        output.push_str(wrapped.join(",").as_str());
        output.push(']');
        output
    }
}

impl Searcher {
    pub fn create_query(
        &self,
        searched_terms: Vec<Vec<String>>,
        query_parts: Vec<QueryPart>,
        result_heap: BinaryHeap<DocResult>,
        result_limit: Option<u32>,
    ) -> Query {
        Query {
            searched_terms,
            query_parts,
            result_heap,
            results_retrieved: 0,
            result_limit,
        }
    }
}
