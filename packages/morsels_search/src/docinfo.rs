use morsels_common::BitmapDocinfoDicttableReader;

pub struct DocInfo {
    pub doc_length_factors: Vec<f64>,
    pub doc_length_factors_len: u32,
    pub num_docs: u32,
    pub num_fields: usize,
}

impl DocInfo {
    pub fn create(docinfo_rdr: &mut BitmapDocinfoDicttableReader, num_fields: usize) -> DocInfo {
        // num_docs =/= doc_length_factors.len() due to incremental indexing
        let mut num_docs = 0;
        let mut doc_id_counter = 0;
        let mut avg_doc_lengths: Vec<f64> = Vec::new();
        docinfo_rdr.read_docinfo_inital_metadata(&mut num_docs, &mut doc_id_counter, &mut avg_doc_lengths, num_fields);

        let mut doc_length_factors: Vec<f64> = Vec::with_capacity(num_fields * doc_id_counter as usize);

        let mut doc_id = 0;
        while doc_id < doc_id_counter {
            doc_id += 1;

            for avg_doc_length in avg_doc_lengths.iter() {
                let field_length = docinfo_rdr.read_field_length() as f64;
                doc_length_factors.push(field_length / *avg_doc_length);
            }
        }

        let doc_length_factors_len = doc_length_factors.len() as u32;
        DocInfo { doc_length_factors, doc_length_factors_len, num_docs, num_fields }
    }

    #[inline(always)]
    pub fn get_doc_length_factor(&self, doc_id: usize, field_id: usize) -> f32 {
        self.doc_length_factors[(doc_id * self.num_fields) + field_id] as f32
    }
}
