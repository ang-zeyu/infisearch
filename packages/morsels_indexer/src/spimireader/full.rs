use std::collections::BinaryHeap;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use dashmap::DashMap;

use crate::docinfo::DocInfos;
use crate::fieldinfo::FieldInfos;
use crate::i_debug;
use crate::spimireader::common::{
    self, postings_stream::PostingsStream, terms, PostingsStreamDecoder, TermDocsForMerge,
};
use crate::utils::varint;
use crate::IncrementalIndexInfo;
use crate::MainToWorkerMessage;
use crate::MorselsIndexingConfig;
use crate::Receiver;
use crate::Sender;

#[allow(clippy::too_many_arguments)]
pub fn merge_blocks(
    doc_id_counter: u32,
    num_blocks: u32,
    first_block: u32,
    last_block: u32,
    indexing_config: &MorselsIndexingConfig,
    field_infos: &Arc<FieldInfos>,
    doc_infos: Arc<Mutex<DocInfos>>,
    tx_main: &Sender<MainToWorkerMessage>,
    output_folder_path: &Path,
    mut docinfo_dicttable_writer: BufWriter<File>,
    incremental_info: &mut IncrementalIndexInfo,
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
        doc_infos_unwrapped_inner.finalize_and_flush(
            &mut docinfo_dicttable_writer,
            doc_id_counter, field_infos.num_scored_fields,
            incremental_info
        );

        Arc::from(doc_infos_unwrapped_inner)
    };

    common::initialise_postings_stream_readers(
        first_block,
        last_block,
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

    let mut dict_string_writer = common::get_dictstring_writer(output_folder_path);
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
            Some(&mut docinfo_dicttable_writer),
            &mut curr_pl,
            &mut pl_writer,
            &mut curr_pl_offset,
            &mut prev_pl_start_offset,
            doc_freq,
            curr_term_max_score,
            num_docs_double,
            &mut incremental_info.pl_names_to_cache,
            indexing_config,
            output_folder_path,
        );

        // ---------------------------------------------

        // ---------------------------------------------
        // Dictionary table writing: doc freq (var-int), pl offset (var-int)

        docinfo_dicttable_writer.write_all(varint::get_var_int(doc_freq, &mut varint_buf)).unwrap();

        docinfo_dicttable_writer
            .write_all(varint::get_var_int(start_pl_offset - prev_pl_start_offset, &mut varint_buf))
            .unwrap();
        prev_pl_start_offset = start_pl_offset;

        // ---------------------------------------------
        // Dictionary string writing

        terms::frontcode_and_store_term(&prev_term, &curr_term, &mut dict_string_writer);

        prev_term = curr_term;

        // ---------------------------------------------
    }

    pl_writer.flush(curr_pl_offset, indexing_config.pl_cache_threshold, &mut incremental_info.pl_names_to_cache);

    docinfo_dicttable_writer.flush().unwrap();
    dict_string_writer.flush().unwrap();

    incremental_info.last_pl_number = if curr_pl_offset != 0 || curr_pl == 0 {
        curr_pl
    } else {
        curr_pl - 1
    };
}
