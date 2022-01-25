use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use rustc_hash::FxHashMap;

use crate::i_debug;
use crate::worker::miner::TermDoc;
use crate::worker::miner::TermDocComparator;


pub fn write_block(combined_terms: FxHashMap<String, Vec<Vec<TermDoc>>>, output_folder_path: PathBuf, block_number: u32) {
    let mut combined_terms_vec: Vec<_> = combined_terms.into_iter().collect();
    combined_terms_vec.sort_by(|a, b| a.0.cmp(&b.0));
    let dict_output_file_path = output_folder_path.join(format!("bsbi_block_dict_{}", block_number));
    let output_file_path = output_folder_path.join(format!("bsbi_block_{}", block_number));

    i_debug!(
        "Writing bsbi block {} to {}, num terms {}",
        block_number,
        output_file_path.to_str().unwrap(),
        combined_terms_vec.len()
    );

    let df = File::create(dict_output_file_path)
        .expect("Failed to open temporary dictionary table for writing.");
    let mut buffered_writer_dict = BufWriter::new(df);
    let f = File::create(output_file_path).expect("Failed to open temporary block file for writing.");
    let mut buffered_writer = BufWriter::with_capacity(819200, f);
    for (term, workers_term_docs) in combined_terms_vec {
        buffered_writer_dict.write_all(&(term.len() as u8).to_le_bytes()).unwrap();
        buffered_writer_dict.write_all(term.as_bytes()).unwrap();
        let mut doc_freq = 0;

        // Initialise heap sort
        let mut heap: BinaryHeap<TermDocComparator> = BinaryHeap::new();
        for term_docs in workers_term_docs {
            doc_freq += term_docs.len() as u32;
            let mut iter = term_docs.into_iter();
            if let Some(term_doc) = iter.next() {
                heap.push(TermDocComparator(term_doc, iter));
            }
        }

        buffered_writer_dict.write_all(&doc_freq.to_le_bytes()).unwrap();

        while !heap.is_empty() {
            let mut term_doc_and_iter = heap.pop().unwrap();

            buffered_writer.write_all(&term_doc_and_iter.0.doc_id.to_le_bytes()).unwrap();

            let num_fields = term_doc_and_iter.0.doc_fields
                .iter()
                .filter(|doc_field| doc_field.field_tf > 0)
                .count() as u8;
            buffered_writer.write_all(&[num_fields]).unwrap();

            for (field_id, doc_field) in term_doc_and_iter.0.doc_fields.into_iter().enumerate() {
                if doc_field.field_tf == 0 {
                    continue;
                }

                buffered_writer.write_all(&[field_id as u8]).unwrap();
                buffered_writer.write_all(&doc_field.field_tf.to_le_bytes()).unwrap();

                for pos in doc_field.positions {
                    buffered_writer.write_all(&pos.to_le_bytes()).unwrap();
                }
            }

            if let Some(term_doc) = term_doc_and_iter.1.next() {
                heap.push(TermDocComparator(term_doc, term_doc_and_iter.1));
            }
        }
    }

    buffered_writer.flush().unwrap();
    buffered_writer_dict.flush().unwrap();
}
