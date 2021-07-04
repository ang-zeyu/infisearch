use crate::worker::miner::WorkerMinerDocInfo;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;


pub struct DocInfos {
    doc_lengths: Vec<WorkerMinerDocInfo>,
    total_lengths: Vec<u64>,
}

impl DocInfos {
    pub fn get_field_len_factor(&self, doc_id: usize, field_id: usize) -> f32 {
        (self.doc_lengths[doc_id].field_lengths[field_id]) as f32 / self.total_lengths[field_id] as f32
    }

    pub fn init_doc_infos(num_scored_fields: usize) -> DocInfos {
        DocInfos {
            doc_lengths: Vec::new(),
            total_lengths: vec![0; num_scored_fields],
        }
    }

    pub fn extend_with(&mut self, sorted_doc_lengths: Vec<WorkerMinerDocInfo>) {
        for worker_miner_doc_info in sorted_doc_lengths.iter() {
            for (field_id, field_length) in worker_miner_doc_info.field_lengths.iter().enumerate() {
                *self.total_lengths.get_mut(field_id).unwrap() += (*field_length) as u64;
            }
        }

        self.doc_lengths.extend(sorted_doc_lengths);
    }

    pub fn divide_field_lengths(&mut self) {
        let num_docs = self.doc_lengths.len() as u64;
        for total_length in self.total_lengths.iter_mut() {
            *total_length /= num_docs;
        }
    }

    pub fn flush(&mut self, output_file_path: PathBuf) {
        let mut doc_info_writer = BufWriter::new(File::create(output_file_path).unwrap());

        doc_info_writer.write_all(&(self.doc_lengths.len() as u32).to_le_bytes()).unwrap();

        for total_length in self.total_lengths.iter() {
            doc_info_writer.write_all(&(*total_length as u32).to_le_bytes()).unwrap();
        }

        for worker_miner_doc_info in self.doc_lengths.iter() {
            for field_length in worker_miner_doc_info.field_lengths.iter() {
                doc_info_writer.write_all(&field_length.to_le_bytes()).unwrap();
            }
        }

        doc_info_writer.flush().unwrap();
    }
}