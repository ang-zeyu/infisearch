use crate::{searcher::Searcher, postings_list::{Doc, PostingsList}};


impl Searcher {
    /*
     "Soft" disjunctive maximum
     Fields are split into 2 groups: "major" / "minor", with a hardcoded (for now) weight to each.

     The major group contains the highest scoring field, while the minor ones contain the rest,
     which share the 0.3 proportion of the score.
     This avoids penalizing documents that don't have the search term in all fields overly heavily,
     while encouraging matches in multiple fields to some degree.
    */
    pub fn calc_doc_bm25_score(&self, td: &Doc, doc_id: u32, pl: &PostingsList, weight: f32) -> f32 {
        const MAJOR_FIELD_FACTOR: f32 = 0.7;
        const MINOR_FIELD_FACTOR: f32 = 0.3;

        let mut doc_term_score = 0.0;
        let mut highest_field_score = 0.0;

        for (field_id, field) in td.fields.iter().enumerate() {
            if field.field_tf > 0.0 {
                debug_assert!(field_id < self.searcher_config.num_scored_fields);
                let field_info = unsafe { self.searcher_config.field_infos.get_unchecked(field_id) };
                let field_len_factor = self.doc_info.get_doc_length_factor(doc_id as usize, field_id as usize);

                let field_score = ((field.field_tf * (field_info.k + 1.0))
                    / (field.field_tf
                        + field_info.k * (1.0 - field_info.b + field_info.b * field_len_factor)))
                    * field_info.weight;

                if field_score > highest_field_score {
                    highest_field_score = field_score;
                }
                doc_term_score += field_score;
            }
        }

        let minor_fields_score = (doc_term_score - highest_field_score) / self.num_scored_fields_less_one;
        ((MINOR_FIELD_FACTOR * minor_fields_score) + (MAJOR_FIELD_FACTOR * highest_field_score)) * pl.idf as f32 * weight
    }
}
