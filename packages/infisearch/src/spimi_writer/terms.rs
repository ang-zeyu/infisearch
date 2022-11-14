use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::i_debug;
use crate::worker::miner::TermDoc;
use crate::worker::miner::TermDocComparator;


pub fn write_block(
    mut combined_terms: Vec<(String, Vec<TermDoc>)>,
    output_folder_path: PathBuf,
    block_number: u32,
) {
    combined_terms.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    let dict_output_file_path = output_folder_path.join(format!("bsbi_block_dict_{}", block_number));
    let output_file_path = output_folder_path.join(format!("bsbi_block_{}", block_number));

    i_debug!(
        "Writing bsbi block {} to {}, num terms {}",
        block_number,
        output_file_path.to_str().unwrap(),
        combined_terms.len()
    );

    let mut buffered_writer_dict = BufWriter::new(
        File::create(dict_output_file_path).unwrap(),
    );
    let mut buffered_writer = BufWriter::with_capacity(
        819200,
        File::create(output_file_path).unwrap(),
    );

    let mut curr_term = String::new();
    let mut curr_term_termdocs: Vec<Vec<TermDoc>> = Vec::new();

    for (term, worker_term_docs) in combined_terms {
        if term == curr_term {
            curr_term_termdocs.push(worker_term_docs);
        } else {
            if !curr_term_termdocs.is_empty() {
                write_term(&mut buffered_writer_dict, &mut buffered_writer, curr_term, &mut curr_term_termdocs);
            }
    
            curr_term = term;
            curr_term_termdocs.push(worker_term_docs);
        }
    }

    write_term(&mut buffered_writer_dict, &mut buffered_writer, curr_term, &mut curr_term_termdocs);

    buffered_writer.flush().unwrap();
    buffered_writer_dict.flush().unwrap();
}


fn write_term(
    buffered_writer_dict: &mut BufWriter<File>,
    buffered_writer: &mut BufWriter<File>,
    term: String,
    curr_term_termdocs: &mut Vec<Vec<TermDoc>>,
) {
    buffered_writer_dict.write_all(&(term.len() as u8).to_le_bytes()).unwrap();
    buffered_writer_dict.write_all(term.as_bytes()).unwrap();

    let mut doc_freq = 0;
    // Initialise heap sort
    let mut heap: BinaryHeap<TermDocComparator> = BinaryHeap::with_capacity(curr_term_termdocs.len());
    for term_docs in curr_term_termdocs.drain(..) {
        doc_freq += term_docs.len() as u32;
        let mut iter = term_docs.into_iter();
        if let Some(term_doc) = iter.next() {
            heap.push(TermDocComparator(term_doc, iter));
        }
    }

    buffered_writer_dict.write_all(&doc_freq.to_le_bytes()).unwrap();
    while let Some(TermDocComparator(term_doc, mut iter)) = heap.pop() {
        buffered_writer.write_all(&term_doc.doc_id.to_le_bytes()).unwrap();

        let num_fields = term_doc.doc_fields
            .iter()
            .filter(|doc_field| doc_field.field_tf > 0)
            .count() as u8;
        buffered_writer.write_all(&[num_fields]).unwrap();

        for (field_id, doc_field) in term_doc.doc_fields.into_iter().enumerate() {
            if doc_field.field_tf == 0 {
                continue;
            }

            buffered_writer.write_all(&[field_id as u8]).unwrap();
            buffered_writer.write_all(&doc_field.field_tf.to_le_bytes()).unwrap();

            for pos in doc_field.positions {
                buffered_writer.write_all(&pos.to_le_bytes()).unwrap();
            }
        }

        if let Some(term_doc) = iter.next() {
            heap.push(TermDocComparator(term_doc, iter));
        }
    }
}
