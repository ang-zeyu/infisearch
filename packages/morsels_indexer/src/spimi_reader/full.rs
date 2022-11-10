use std::collections::BinaryHeap;
use std::path::Path;
use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender};
use dashmap::DashMap;

use crate::dictionary_writer::DictWriter;
use crate::field_info::FieldInfos;
use crate::incremental_info::IncrementalIndexInfo;
use crate::indexer::input_config::MorselsIndexingConfig;
use crate::i_debug;
use crate::spimi_reader::common::{
    self, postings_stream::PostingsStream, PostingsStreamDecoder, TermDocsForMerge,
};
use crate::worker::MainToWorkerMessage;

#[allow(clippy::too_many_arguments)]
pub fn merge_blocks(
    has_docs_added: bool,
    num_blocks: u32,
    first_block: u32,
    last_block: u32,
    indexing_config: &MorselsIndexingConfig,
    field_infos: &Arc<FieldInfos>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path,
    incremental_info: &mut IncrementalIndexInfo,
) -> DictWriter {
    /*
    Merges the intermediate results written earlier.
    Each block of intermediate results is abstracted by a "postings stream".

    Whenever a postings stream's primary buffer depletes below a certain count,
    request a worker to decode more terms and postings lists into a secondary buffer.

    Once the primary buffer is fully depleted, wait for the decoding to complete **if not yet done**,
    then swap the two buffers.

    Keep track of postings streams being decoded by threads (secondary buffers being filled)
    using a concurrent HashMap (DashMap).
    */
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();
    let postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>> =
        Arc::from(DashMap::with_capacity(num_blocks as usize));
    let (blocking_sndr, blocking_rcvr): (Sender<()>, Receiver<()>) = crossbeam::channel::bounded(1);

    if !has_docs_added {
        common::initialise_postings_stream_readers(
            first_block,
            last_block,
            output_folder_path,
            &mut postings_streams,
            &postings_stream_decoders,
            field_infos.num_scored_fields,
            tx_main,
            &blocking_sndr,
            &blocking_rcvr,
        );
    }

    /*
    N-way merge according to lexicographical order
    Sort and aggregate worker docIds into one vector
    */

    let mut dict_writer = DictWriter::new();
    let mut pl_writer = common::get_pl_writer(output_folder_path, 0, indexing_config.num_pls_per_dir);

    // Preallocate some things
    let mut curr_combined_term_docs: Vec<TermDocsForMerge> = Vec::with_capacity(num_blocks as usize);

    // Dictionary front coding tracker
    let mut prev_term = "".to_owned();

    // Dictionary table / Postings list trackers
    let mut curr_pl = 0;
    let mut curr_pl_offset: u32 = 0;
    let mut prev_pl_start_offset: u32 = 0;

    // Varint buffer
    let mut varint_buf: [u8; 16] = [0; 16];

    i_debug!("Starting main decode loop...! Number of blocks {}", postings_streams.len());

    while !postings_streams.is_empty() {
        let (curr_term, doc_freq) = PostingsStream::aggregate_block_terms(
            &mut curr_combined_term_docs,
            &mut postings_streams,
            &postings_stream_decoders,
            tx_main,
            &blocking_sndr,
            &blocking_rcvr,
        );

        // Commit the term's n-way merged postings (curr_combined_term_docs),
        // and dictionary table, dictionary-as-a-string for the term.

        // ---------------------------------------------
        // Postings writing

        // Postings

        let start_pl_offset = common::write_new_term_postings(
            &mut curr_combined_term_docs,
            &mut varint_buf,
            Some(&mut dict_writer),
            &mut curr_pl,
            &mut pl_writer,
            &mut curr_pl_offset,
            &mut prev_pl_start_offset,
            &mut incremental_info.pl_names_to_cache,
            indexing_config,
            output_folder_path,
        );

        // ---------------------------------------------

        // ---------------------------------------------
        // Dictionary writing

        let (prefix_len, remaining_len) = dict_writer.write_term(
            &prev_term, &curr_term,
        );

        dict_writer.write_dict_table_entry(
            doc_freq, start_pl_offset, &mut prev_pl_start_offset,
            prefix_len, remaining_len,
        );

        prev_term = curr_term;

        // ---------------------------------------------
    }

    pl_writer.flush(curr_pl_offset, indexing_config.pl_cache_threshold, &mut incremental_info.pl_names_to_cache);

    incremental_info.last_pl_number = if curr_pl_offset != 0 || curr_pl == 0 {
        curr_pl
    } else {
        curr_pl - 1
    };

    dict_writer
}
