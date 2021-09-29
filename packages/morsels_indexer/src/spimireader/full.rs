use std::collections::BinaryHeap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use dashmap::DashMap;

use morsels_common::DOC_INFO_FILE_NAME;

use crate::docinfo::DocInfos;
use crate::utils::varint;
use crate::DynamicIndexInfo;
use crate::MainToWorkerMessage;
use crate::MorselsIndexingConfig;
use crate::Receiver;
use crate::Sender;
use crate::spimireader::common::{
    self,
    postings_stream::PostingsStream,
    PostingsStreamDecoder,
    TermDocsForMerge,
    terms,
};

#[allow(clippy::too_many_arguments)]
pub fn merge_blocks(
    doc_id_counter: u32,
    num_blocks: u32,
    indexing_config: &MorselsIndexingConfig,
    pl_names_to_cache: &mut Vec<u32>,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path,
    dynamic_index_info: &mut DynamicIndexInfo,
) {
    /*
    Gist of this function:

    Whenever a postings stream's primary buffer depletes below a certain count,
    request a worker to decode more terms and postings lists into the secondary buffer.

    Once the primary buffer is fully depleted, wait for the decoding to complete **if not yet done**, then swap the two buffers.

    We keep track of postings streams being decoded by threads... (secondary buffers being filled)
    using a concurrent HashMap (DashMap)...
    */
    let mut postings_streams: BinaryHeap<PostingsStream> = BinaryHeap::new();
    let postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>> =
        Arc::from(DashMap::with_capacity(num_blocks as usize));
    let (blocking_sndr, blocking_rcvr): (Sender<()>, Receiver<()>) = crossbeam::channel::bounded(1);

    let num_docs_double = doc_id_counter as f64;

    // Unwrap the inner mutex to avoid locks as it is now read-only
    let doc_infos_unlocked_arc = {
        let mut doc_infos_unwrapped_inner = Arc::try_unwrap(doc_infos)
            .expect("No thread should be holding doc infos arc when merging blocks")
            .into_inner()
            .expect("No thread should be holding doc infos mutex when merging blocks");
        doc_infos_unwrapped_inner.finalize_and_flush(output_folder_path.join(DOC_INFO_FILE_NAME), doc_id_counter);

        Arc::from(doc_infos_unwrapped_inner)
    };

    common::initialise_postings_streams(
        num_blocks,
        output_folder_path,
        &mut postings_streams,
        &postings_stream_decoders,
        &doc_infos_unlocked_arc,
        tx_main,
        &blocking_sndr,
        &blocking_rcvr,
    );

    /*
    N-way merge according to lexicographical order
    Sort and aggregate worker docIds into one vector
    */

    let (mut dict_table_writer, mut dict_string_writer) = common::get_dict_writers(output_folder_path);
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

    #[cfg(debug_assertions)]
    println!("Starting main decode loop...! Number of blocks {}", postings_streams.len());

    while !postings_streams.is_empty() {
        let (curr_term, doc_freq, curr_term_max_score) = PostingsStream::aggregate_block_terms(
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
            Some(&mut dict_table_writer),
            &mut curr_pl,
            &mut pl_writer,
            &mut curr_pl_offset,
            &mut prev_pl_start_offset,
            doc_freq,
            curr_term_max_score,
            num_docs_double,
            pl_names_to_cache,
            indexing_config,
            output_folder_path,
        );

        // ---------------------------------------------

        // ---------------------------------------------
        // Dictionary table writing: doc freq (var-int), pl offset (var-int)

        dict_table_writer.write_all(varint::get_var_int(doc_freq, &mut varint_buf)).unwrap();

        dict_table_writer.write_all(varint::get_var_int(start_pl_offset - prev_pl_start_offset, &mut varint_buf)).unwrap();
        prev_pl_start_offset = start_pl_offset;

        // ---------------------------------------------
        // Dictionary string writing

        terms::frontcode_and_store_term(&prev_term, &curr_term, &mut dict_string_writer);

        prev_term = curr_term;

        // ---------------------------------------------
    }

    dynamic_index_info.last_pl_number = if curr_pl_offset != 0 || curr_pl == 0 { curr_pl } else { curr_pl - 1 };
    dynamic_index_info.num_docs = doc_id_counter;

    dict_table_writer.flush().unwrap();
    pl_writer.flush().unwrap();
    dict_string_writer.flush().unwrap();
}
