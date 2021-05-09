use std::convert::TryInto;

struct DocInfo {
    normalization_factors: Vec<f64>
}

impl DocInfo {
    fn sqrt_normalization_factors(&mut self) {
        for i in 1..self.normalization_factors.len() {
            self.normalization_factors[i] = self.normalization_factors[i].sqrt();
        }
    }

    fn get_dump_string(&self) -> String {
        let mut buffer: Vec<String> = Vec::new();
        for i in 1..self.normalization_factors.len() {
            buffer.push(format!("{:.6}", self.normalization_factors[i]));
        }

        buffer.join("\n")
    }
}

pub struct DocInfos {
    doc_infos: Vec<DocInfo>
}

impl DocInfos {
    pub fn add_doc_len(&mut self, doc_id: u32, field_id: u8, tf_idf: f64) {
        let doc_id_usize: usize = doc_id.try_into().unwrap();
        let field_id_usize: usize = field_id.try_into().unwrap();

        for i in self.doc_infos.len()..doc_id_usize {
            self.doc_infos.push(DocInfo { normalization_factors: Vec::new() })
        }

        let doc_info: &mut DocInfo = &mut self.doc_infos[doc_id_usize];
        for j in doc_info.normalization_factors.len()..field_id_usize {
            doc_info.normalization_factors.push(0.0);
        }

        let field_len = doc_info.normalization_factors.get_mut(field_id_usize).unwrap();
        *field_len += tf_idf * tf_idf;
    }
    
    fn sqrt_normalization_factors(&mut self) {
        for i in 1..self.doc_infos.len() {
            self.doc_infos[i].sqrt_normalization_factors();
        }
    }
    
    pub fn get_dump_string(&self) -> String {
      let mut buffer: Vec<String> = Vec::new();
      for i in 1..self.doc_infos.len() {
        buffer.push(self.doc_infos[i].get_dump_string());
      }

      buffer.join("\n")
    }
}