use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use rustc_hash::FxHashMap;

use crate::docinfo::DocInfos;
use crate::docinfo::BlockDocLengths;
use crate::fieldinfo::FieldInfos;
use crate::i_debug;
use crate::worker::miner::DocIdAndFieldLengthsComparator;
use crate::worker::miner::TermDoc;
use crate::worker::miner::WorkerBlockIndexResults;
use crate::worker::miner::WorkerMinerDocInfo;

mod fields;
mod write_block;

#[allow(clippy::too_many_arguments)]
pub fn combine_worker_results_and_write_block(
    worker_index_results: Vec<WorkerBlockIndexResults>,
    doc_infos: Arc<Mutex<DocInfos>>,
    output_folder_path: PathBuf,
    field_infos: &Arc<FieldInfos>,
    block_number: u32,
    start_doc_id: u32,
    check_for_existing_field_store: bool,
    num_docs_per_block: u32,
    spimi_counter: u32,
    doc_id_counter: u32,
) {
    let mut combined_terms: FxHashMap<String, Vec<Vec<TermDoc>>> = FxHashMap::default();

    let mut heap: BinaryHeap<DocIdAndFieldLengthsComparator> = BinaryHeap::with_capacity(worker_index_results.len());

    // Combine
    for worker_result in worker_index_results.into_iter().filter(|w| !w.doc_infos.is_empty()) {
        for (worker_term, worker_term_docs) in worker_result.terms {
            combined_terms.entry(worker_term).or_insert_with(Vec::new).push(worker_term_docs);
        }

        let mut doc_infos_iter = worker_result.doc_infos.into_iter();
        if let Some(worker_document_length) = doc_infos_iter.next() {
            heap.push(DocIdAndFieldLengthsComparator(worker_document_length, doc_infos_iter));
        }
    }

    {
        let mut sorted_doc_infos: Vec<WorkerMinerDocInfo> = Vec::with_capacity(spimi_counter as usize);

        // ---------------------------------------------
        // Heap sort by doc id
        while !heap.is_empty() {
            let mut top = heap.pop().unwrap();

            if let Some(worker_document_length) = top.1.next() {
                heap.push(DocIdAndFieldLengthsComparator(worker_document_length, top.1));
            }

            sorted_doc_infos.push(top.0);
        }
        // ---------------------------------------------

        // ---------------------------------------------
        // Store field texts
        if !sorted_doc_infos.is_empty() {
            fields::store_fields(
                check_for_existing_field_store,
                start_doc_id,
                field_infos,
                doc_id_counter,
                spimi_counter,
                num_docs_per_block,
                block_number,
                &mut sorted_doc_infos
            );

            i_debug!("Num docs in block {}: {}", block_number, sorted_doc_infos.len());
        } else {
            // possibly just a incremental indexing run with a deletion
            i_debug!("Encountered empty block {}", block_number);
        }
        // ---------------------------------------------

        if !sorted_doc_infos.is_empty() {
            doc_infos.lock().unwrap().all_block_doc_lengths.push(BlockDocLengths(sorted_doc_infos));
        }
    }

    write_block::write_block(combined_terms, output_folder_path, block_number);
}


