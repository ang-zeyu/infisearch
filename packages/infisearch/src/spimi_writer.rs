use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use crate::doc_info::DocInfos;
use crate::doc_info::BlockDocLengths;
use crate::field_info::FieldInfos;
use crate::i_debug;
use crate::utils::time;
use crate::worker::miner::DocIdAndFieldLengthsComparator;
use crate::worker::miner::TermDoc;
use crate::worker::miner::WorkerBlockIndexResults;
use crate::worker::miner::WorkerMinerDocInfo;

mod fields;
mod terms;

#[allow(clippy::too_many_arguments)]
pub fn combine_worker_results_and_write_block(
    worker_index_results: Vec<WorkerBlockIndexResults>,
    doc_infos: Arc<Mutex<DocInfos>>,
    output_folder_path: PathBuf,
    field_infos: &Arc<FieldInfos>,
    block_number: u32,
    start_doc_id: u32,
    check_for_existing_field_store: bool,
    spimi_counter: u32,
    doc_id_counter: u32,
    log_perf: bool,
) {
    let now = if log_perf { Some(Instant::now()) } else { None };

    let mut combined_terms: Vec<(String, Vec<TermDoc>)> = Vec::with_capacity(
        worker_index_results.iter().map(|result| result.terms.len()).sum()
    );

    let mut heap: BinaryHeap<DocIdAndFieldLengthsComparator> = BinaryHeap::with_capacity(worker_index_results.len());

    // Combine all (String, Vec<TermDoc>) pairs into one vector, and initialise docinfos heapsort
    for worker_result in worker_index_results.into_iter().filter(|w| !w.doc_infos.is_empty()) {
        combined_terms.extend(worker_result.terms);

        let mut doc_infos_iter = worker_result.doc_infos.into_iter();
        if let Some(worker_document_length) = doc_infos_iter.next() {
            heap.push(DocIdAndFieldLengthsComparator(worker_document_length, doc_infos_iter));
        }
    }

    {
        let mut sorted_doc_infos: Vec<WorkerMinerDocInfo> = Vec::with_capacity(spimi_counter as usize);

        // ---------------------------------------------
        // Heap sort by doc id
        while let Some(DocIdAndFieldLengthsComparator(worker_document_length, mut iter)) = heap.pop() {
            if let Some(worker_document_length) = iter.next() {
                heap.push(DocIdAndFieldLengthsComparator(worker_document_length, iter));
            }

            sorted_doc_infos.push(worker_document_length);
        }

        if log_perf {
            time::print_time_elapsed(&now, &format!("block {} sorted", block_number));
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
                block_number,
                sorted_doc_infos
                    .iter_mut()
                    .map(|doc_info| std::mem::take(&mut doc_info.field_texts))
                    .collect()
            );

            i_debug!("Num docs in block {}: {}", block_number, sorted_doc_infos.len());
        } else {
            // possibly just a incremental indexing run with a deletion
            i_debug!("Encountered empty block {}", block_number);
        }

        if log_perf {
            time::print_time_elapsed(&now, &format!("block {} fields stored", block_number));
        }
        // ---------------------------------------------

        if !sorted_doc_infos.is_empty() {
            doc_infos.lock().unwrap().all_block_doc_lengths.push(BlockDocLengths(sorted_doc_infos));
        }

        if log_perf {
            time::print_time_elapsed(&now, &format!("block {} infos stashed", block_number));
        }
    }

    terms::write_block(combined_terms, output_folder_path, block_number);

    if log_perf {
        time::print_time_elapsed(&now, &format!("block {} written", block_number));
    }
}


