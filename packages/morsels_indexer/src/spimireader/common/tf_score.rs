use std::sync::Arc;

use crate::{fieldinfo::FieldInfos, docinfo::DocInfos};

#[inline(always)]
pub fn add_field_to_doc_score(
    field_infos: &Arc<FieldInfos>,
    field_id: u8,
    curr_doc_term_score: &mut f32,
    field_tf: u32,
    doc_infos: &Arc<DocInfos>,
    doc_id: u32,
) {
    let field_info = field_infos.field_infos_by_id.get(field_id as usize).unwrap();
    let k = field_info.k;
    let b = field_info.b;
    *curr_doc_term_score += ((field_tf as f32 * (k + 1.0))
        / (field_tf as f32
            + k * (1.0 - b
                + b * (doc_infos
                    .get_field_len_factor(doc_id as usize, field_id as usize)))))
        * field_info.weight;
}
