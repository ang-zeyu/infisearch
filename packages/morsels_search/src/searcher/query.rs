use std::cmp::Ordering;
use std::collections::BinaryHeap;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

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
    query_parts: Vec<QueryPart>,
    result_heap: BinaryHeap<DocResult>,
    results_retrieved: u32,
    pub results_total: usize,
    result_limit: Option<u32>,
}

#[wasm_bindgen]
impl Query {
    pub fn get_next_n(&mut self, n: usize) -> Vec<u32> {
        let mut doc_ids: Vec<u32> = Vec::with_capacity(n);
        while !self.result_heap.is_empty()
            && doc_ids.len() < n
            && (self.result_limit.is_none() || self.results_retrieved < unsafe { self.result_limit.unwrap_unchecked() })
        {
            doc_ids.push(unsafe { self.result_heap.pop().unwrap_unchecked().doc_id });
            self.results_retrieved += 1;
        }

        doc_ids
    }

    pub fn get_query_parts(&self) -> String {
        QueryPart::serialize_parts(&self.query_parts)
    }
}

impl Searcher {
    pub fn create_query(
        &self,
        query_parts: Vec<QueryPart>,
        result_heap: BinaryHeap<DocResult>,
        result_limit: Option<u32>,
    ) -> Query {
        let results_total = result_heap.len();
        Query {
            query_parts,
            result_heap,
            results_retrieved: 0,
            results_total,
            result_limit,
        }
    }
}
