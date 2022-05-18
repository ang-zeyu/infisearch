use std::cmp::Ordering;

use crate::incremental_info::IncrementalIndexInfo;
use crate::worker::miner::WorkerMinerDocInfo;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

use morsels_common::BitmapDocinfoDicttableReader;
use morsels_common::bitmap;

#[derive(Debug)]
pub struct BlockDocLengths(pub Vec<WorkerMinerDocInfo>);

impl Eq for BlockDocLengths {}

impl PartialEq for BlockDocLengths {
    fn eq(&self, other: &Self) -> bool {
        self.0[0].doc_id == other.0[0].doc_id
    }
}

impl Ord for BlockDocLengths {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0[0].doc_id.cmp(&other.0[0].doc_id)
    }
}

impl PartialOrd for BlockDocLengths {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0[0].doc_id.cmp(&other.0[0].doc_id))
    }
}

#[derive(Debug)]
pub struct DocInfos {
    pub doc_lengths: Vec<WorkerMinerDocInfo>,
    pub all_block_doc_lengths: Vec<BlockDocLengths>, // store doc lengths from each block and sort later
    average_lengths: Vec<f64>,
}

impl DocInfos {
    pub fn from_search_docinfo(bitmap_docinfo_dicttable: &mut BitmapDocinfoDicttableReader, num_fields: usize) -> DocInfos {
        let mut doc_id_counter = 0;
        let mut average_lengths: Vec<f64> = Vec::new();
        bitmap_docinfo_dicttable.read_docinfo_inital_metadata(&mut 0, &mut doc_id_counter, &mut average_lengths, num_fields);

        let mut doc_lengths = Vec::new();
        let mut doc_id = 0;
        while doc_id < doc_id_counter {
            let mut doc_info = WorkerMinerDocInfo {
                doc_id,
                field_lengths: Vec::with_capacity(num_fields),
                field_texts: Vec::new(),
            };
            doc_id += 1;

            for _i in 0..num_fields {
                doc_info.field_lengths.push(bitmap_docinfo_dicttable.read_field_length());
            }

            doc_lengths.push(doc_info);
        }

        DocInfos { doc_lengths, all_block_doc_lengths: Vec::new(), average_lengths }
    }

    pub fn init_doc_infos(num_scored_fields: usize) -> DocInfos {
        DocInfos {
            doc_lengths: Vec::new(),
            all_block_doc_lengths: Vec::new(),
            average_lengths: vec![0.0; num_scored_fields],
        }
    }

    fn sort_and_merge_block_doclengths(&mut self) {
        self.all_block_doc_lengths.sort();

        self.doc_lengths.extend(
            std::mem::take(&mut self.all_block_doc_lengths)
                .into_iter()
                .flat_map(|block_doc_lengths| block_doc_lengths.0),
        );
    }

    fn calculate_field_average_lengths(
        &mut self,
        writer: &mut BufWriter<std::fs::File>,
        num_docs: u32,
        num_scored_fields: usize,
        incremental_info: &mut IncrementalIndexInfo,
    ) {
        let mut total_field_lengths: Vec<u64> = vec![0; num_scored_fields];
        for (doc_id, worker_miner_doc_info) in self.doc_lengths.iter().enumerate() {
            if !bitmap::check(&incremental_info.invalidation_vector, doc_id) {
                for (field_id, field_length) in worker_miner_doc_info.field_lengths.iter().enumerate() {
                    *total_field_lengths.get_mut(field_id).unwrap() += (*field_length) as u64;
                }
            }
        }

        let num_docs = num_docs as f64;
        for (field_id, total_length) in total_field_lengths.into_iter().enumerate() {
            let average_length = self.average_lengths.get_mut(field_id).unwrap();
            *average_length = total_length as f64 / num_docs;
            writer.write_all(&(*average_length).to_le_bytes()).unwrap();
        }
    }

    pub fn finalize_and_flush(
        &mut self,
        doc_info_writer: &mut BufWriter<File>,
        num_docs: u32,
        num_scored_fields: usize,
        incremental_info: &mut IncrementalIndexInfo,
    ) {
        self.sort_and_merge_block_doclengths();

        doc_info_writer.write_all(&num_docs.to_le_bytes()).unwrap();
        
        let doc_lengths_len = self.doc_lengths.len() as u32;
        doc_info_writer.write_all(&doc_lengths_len.to_le_bytes()).unwrap();

        self.calculate_field_average_lengths(doc_info_writer, num_docs, num_scored_fields, incremental_info);

        for worker_miner_doc_info in self.doc_lengths.iter() {
            for field_length in worker_miner_doc_info.field_lengths.iter() {
                doc_info_writer.write_all(&field_length.to_le_bytes()).unwrap();
            }
        }
    }
}

impl Default for DocInfos {
    fn default() -> Self {
        DocInfos { doc_lengths: Vec::new(), all_block_doc_lengths: Vec::new(), average_lengths: vec![0.0; 0] }
    }
}
