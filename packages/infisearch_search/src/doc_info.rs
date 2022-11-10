use infisearch_common::{MetadataReader, EnumMax};

pub struct DocInfo {
    pub doc_length_factors: Vec<f64>,
    pub doc_length_factors_len: u32,
    pub doc_enum_vals: Vec<EnumMax>,
    pub num_docs: u32,
    pub num_fields: usize,
    pub num_enum_fields: usize,
}

impl DocInfo {
    pub fn create(docinfo_rdr: &mut MetadataReader, num_fields: usize) -> DocInfo {
        // num_docs =/= doc_length_factors.len() due to incremental indexing
        let mut num_docs = 0;
        let mut doc_id_counter = 0;
        let mut avg_doc_lengths: Vec<f64> = Vec::new();
        let mut num_enum_fields = 0;

        let doc_enum_vals = docinfo_rdr.read_docinfo_inital_metadata(
            &mut num_docs,
            &mut doc_id_counter,
            &mut avg_doc_lengths,
            &mut num_enum_fields,
            num_fields
        );

        let mut doc_length_factors: Vec<f64> = Vec::with_capacity(num_fields * doc_id_counter as usize);

        for _doc_id in 0..doc_id_counter {
            for avg_doc_length in avg_doc_lengths.iter() {
                let field_length = docinfo_rdr.read_docinfo_field_length() as f64;
                doc_length_factors.push(field_length / *avg_doc_length);
            }
        }

        DocInfo {
            doc_length_factors,
            doc_length_factors_len: doc_id_counter,
            doc_enum_vals,
            num_docs,
            num_fields,
            num_enum_fields,
        }
    }

    #[inline(always)]
    pub fn get_doc_length_factor(&self, doc_id: usize, field_id: usize) -> f32 {
        debug_assert!(((doc_id * self.num_fields) + field_id) < self.doc_length_factors.len());

        (unsafe {
            *self.doc_length_factors.get_unchecked((doc_id * self.num_fields) + field_id)
        }) as f32
    }

    #[inline(always)]
    pub fn get_enum_val(&self, doc_id: usize, enum_id: usize) -> EnumMax {
        debug_assert!(((doc_id * self.num_enum_fields) + enum_id) < self.doc_enum_vals.len());

        unsafe {
            *self.doc_enum_vals.get_unchecked((doc_id * self.num_enum_fields) + enum_id)
        }
    }
}
