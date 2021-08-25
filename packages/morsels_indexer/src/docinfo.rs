use std::cmp::Ordering;

use crate::worker::miner::WorkerMinerDocInfo;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

use byteorder::{ByteOrder, LittleEndian};

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
    pub fn from_search_docinfo(doc_info_vec: Vec<u8>, num_fields: usize) -> DocInfos {
        let mut byte_offset = 4; // first 4 bytes is number of documents

        let mut average_lengths: Vec<f64> = Vec::new();
        for _i in 0..num_fields {
            average_lengths.push(LittleEndian::read_f64(&doc_info_vec[byte_offset..]));
            byte_offset += 8;
        }

        let total_bytes = doc_info_vec.len();
        let mut doc_lengths = Vec::new();
        let mut doc_id = 0;
        while byte_offset < total_bytes {
            let mut doc_info =
                WorkerMinerDocInfo { doc_id, field_lengths: vec![0; num_fields], field_texts: Vec::new() };
            doc_id += 1;

            for i in 0..num_fields {
                doc_info.field_lengths[i] = LittleEndian::read_u32(&doc_info_vec[byte_offset..]);
                byte_offset += 4;
            }

            doc_lengths.push(doc_info);
        }

        DocInfos { doc_lengths, all_block_doc_lengths: Vec::new(), average_lengths }
    }

    pub fn get_field_len_factor(&self, doc_id: usize, field_id: usize) -> f32 {
        ((self.doc_lengths[doc_id].field_lengths[field_id]) as f64 / self.average_lengths[field_id]) as f32
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

    fn calculate_field_average_lengths(&mut self, writer: &mut BufWriter<std::fs::File>) {
        let num_fields = if let Some(first) = self.doc_lengths.get(0) { first.field_lengths.len() } else { 0 };
        let mut total_field_lengths: Vec<u64> = vec![0; num_fields];
        for worker_miner_doc_info in self.doc_lengths.iter() {
            for (field_id, field_length) in worker_miner_doc_info.field_lengths.iter().enumerate() {
                *total_field_lengths.get_mut(field_id).unwrap() += (*field_length) as u64;
            }
        }

        let num_docs = self.doc_lengths.len() as u64;
        for (field_id, total_length) in total_field_lengths.into_iter().enumerate() {
            let average_length = self.average_lengths.get_mut(field_id).unwrap();
            *average_length = total_length as f64 / num_docs as f64;
            writer.write_all(&(*average_length).to_le_bytes()).unwrap();
        }
    }

    pub fn finalize_and_flush(&mut self, output_file_path: PathBuf, num_docs: u32) {
        self.sort_and_merge_block_doclengths();

        let mut doc_info_writer = BufWriter::new(File::create(output_file_path).unwrap());

        doc_info_writer.write_all(&num_docs.to_le_bytes()).unwrap();

        self.calculate_field_average_lengths(&mut doc_info_writer);

        for worker_miner_doc_info in self.doc_lengths.iter() {
            for field_length in worker_miner_doc_info.field_lengths.iter() {
                doc_info_writer.write_all(&field_length.to_le_bytes()).unwrap();
            }
        }

        doc_info_writer.flush().unwrap();
    }
}

impl Default for DocInfos {
    fn default() -> Self {
        DocInfos { doc_lengths: Vec::new(), all_block_doc_lengths: Vec::new(), average_lengths: vec![0.0; 0] }
    }
}
