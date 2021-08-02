use std::cmp::Ordering;

use crate::worker::miner::WorkerMinerDocInfo;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

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

pub struct DocInfos {
    doc_lengths: Vec<WorkerMinerDocInfo>,
    pub all_block_doc_lengths: Vec<BlockDocLengths>, // store doc lengths from each block and sort later
    average_lengths: Vec<f64>,
}

impl DocInfos {
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

        self.doc_lengths = std::mem::take(&mut self.all_block_doc_lengths).into_iter()
            .flat_map(|block_doc_lengths| block_doc_lengths.0)
            .collect();
    }

    fn calculate_field_average_lengths(&mut self, writer: &mut BufWriter<std::fs::File>) {
        for worker_miner_doc_info in self.doc_lengths.iter() {
            for (field_id, field_length) in worker_miner_doc_info.field_lengths.iter().enumerate() {
                *self.average_lengths.get_mut(field_id).unwrap() += (*field_length) as f64;
            }
        }

        let num_docs = self.doc_lengths.len() as u64;
        for total_length in self.average_lengths.iter_mut() {
            *total_length /= num_docs as f64;
            writer.write_all(&(*total_length as u32).to_le_bytes()).unwrap();
        }
    }

    pub fn finalize_and_flush(&mut self, output_file_path: PathBuf) {
        self.sort_and_merge_block_doclengths();

        let mut doc_info_writer = BufWriter::new(File::create(output_file_path).unwrap());

        doc_info_writer.write_all(&(self.doc_lengths.len() as u32).to_le_bytes()).unwrap();

        self.calculate_field_average_lengths(&mut doc_info_writer);

        for worker_miner_doc_info in self.doc_lengths.iter() {
            for field_length in worker_miner_doc_info.field_lengths.iter() {
                doc_info_writer.write_all(&field_length.to_le_bytes()).unwrap();
            }
        }

        doc_info_writer.flush().unwrap();
    }
}