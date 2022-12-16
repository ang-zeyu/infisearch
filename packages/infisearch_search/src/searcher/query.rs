use std::cmp::Ordering;

use binary_heap_plus::BinaryHeap;
use infisearch_common::utils::push;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::doc_info::DocInfo;
use crate::searcher::query_parser::QueryPart;
use crate::searcher::Searcher;

pub struct DocResult {
    pub doc_id: u32,
    pub score: f32,
}

pub type DocResultComparator = dyn Fn(&DocResult, &DocResult) -> Ordering;

#[wasm_bindgen]
pub struct Query {
    query_parts: Vec<QueryPart>,
    result_heap: BinaryHeap<DocResult, Box<DocResultComparator>>,
    results_retrieved: u32,
    pub results_total: usize,
    result_limit: Option<u32>,
    doc_infos: *const DocInfo,
}

#[wasm_bindgen]
impl Query {
    /// Returns the internal doc ids of the next n top ranked documents.
    /// 
    /// Fields are populated on the JS side to avoid significant (de)serialization overheads.
    /// Enum values, if any, are also returned sorted according to enum ids.
    /// 
    /// Format:
    /// doc id 1
    /// enum value for enum_id=0
    /// enum value for enum_id=1
    /// ...
    /// doc id 2
    pub fn get_next_n(&mut self, n: usize) -> Vec<u8> {
        let doc_infos = unsafe { &*self.doc_infos };

        let mut raw: Vec<u8> = Vec::with_capacity(n * (
            4 + doc_infos.num_enum_fields + (8 * doc_infos.num_i64_fields)
        ));

        let mut docs_added = 0;

        while !self.result_heap.is_empty()
            && docs_added < n
            && (self.result_limit.is_none() || self.results_retrieved < unsafe { self.result_limit.unwrap_unchecked() })
        {
            let doc_id = unsafe { self.result_heap.pop().unwrap_unchecked().doc_id };

            push::extend_wo_grow(&mut raw, &doc_id.to_le_bytes());

            let doc_id = doc_id as usize;

            for enum_id in 0..doc_infos.num_enum_fields {
                push::push_wo_grow(
                    &mut raw,
                    doc_infos.get_enum_val(doc_id, enum_id),
                );
            }

            for num_id in 0..doc_infos.num_i64_fields {
                push::extend_wo_grow(
                    &mut raw,
                    &doc_infos.get_num_val(doc_id, num_id).to_le_bytes(),
                );
            }

            docs_added += 1;
            self.results_retrieved += 1;
        }

        raw
    }

    pub fn get_query_parts(&self) -> String {
        QueryPart::serialize_parts(&self.query_parts)
    }
}

impl Searcher {
    pub fn create_query(
        &self,
        query_parts: Vec<QueryPart>,
        result_heap: BinaryHeap<DocResult, Box<DocResultComparator>>,
        result_limit: Option<u32>,
    ) -> Query {
        let results_total = result_heap.len();
        Query {
            query_parts,
            result_heap,
            results_retrieved: 0,
            results_total,
            result_limit,
            doc_infos: (&self.doc_info) as *const DocInfo
        }
    }
}
