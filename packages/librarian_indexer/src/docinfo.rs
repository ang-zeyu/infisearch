use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

struct DocInfo {
    normalization_factors: Vec<f64>
}

impl DocInfo {
    fn sqrt_normalization_factors(&mut self) {
        for i in 1..self.normalization_factors.len() {
            self.normalization_factors[i] = self.normalization_factors[i].sqrt();
        }
    }

    fn write_dump(self, doc_info_writer: &mut BufWriter<File>) {
        for normalization_factor in self.normalization_factors {
            doc_info_writer.write_all(&normalization_factor.to_le_bytes()).unwrap();
        }
    }
}

pub struct DocInfos {
    doc_info_writer: BufWriter<File>,
    doc_infos: Vec<DocInfo>
}

impl DocInfos {
    pub fn init_doc_infos(output_file_path: PathBuf, num_total_docs: usize, num_scored_fields: usize) -> DocInfos {
        let mut doc_infos = Vec::with_capacity(num_total_docs);
        for _i in 0..num_total_docs {
            let mut field_normalization_factors = Vec::with_capacity(num_scored_fields);
            for _j in 0.. num_scored_fields {
                field_normalization_factors.push(0.0);
            }
            doc_infos.push(DocInfo { normalization_factors: field_normalization_factors });
        }

        DocInfos {
            doc_info_writer: BufWriter::new(File::create(output_file_path).unwrap()),
            doc_infos,
        }
    }

    pub fn add_doc_len(&mut self, doc_id: u32, field_id: u8, tf_idf: f64) {
        let doc_info = &mut self.doc_infos[doc_id as usize];
        *doc_info.normalization_factors.get_mut(field_id as usize).unwrap() += tf_idf * tf_idf;
    }

    pub fn flush(mut self) {
        self.doc_info_writer.write_all(&(self.doc_infos.len() as u32).to_le_bytes()).unwrap();

        self.sqrt_normalization_factors();
        for doc_info in self.doc_infos {
            doc_info.write_dump(&mut self.doc_info_writer);
        }

        self.doc_info_writer.flush().unwrap();
    }
    
    fn sqrt_normalization_factors(&mut self) {
        for doc_info in self.doc_infos.iter_mut() {
            doc_info.sqrt_normalization_factors();
        }
    }
}