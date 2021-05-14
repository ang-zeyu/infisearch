use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use crate::fieldinfo::FieldInfo;
use crate::tokenize::english::tokenize;

pub struct DocField {
    pub field_id: u8,
    pub field_positions: Vec<u32>
}

pub struct TermDoc {
    pub doc_id: u32,
    pub doc_fields: Vec<DocField>
}

// Intermediate BSBI miner for use in a worker
// Outputs (termID, docID, fieldId, fieldTf, positions ...., fieldId, fieldTf, positions ....) tuples
pub struct WorkerMiner {
    pub field_infos: Arc<HashMap<String, FieldInfo>>,

    pub terms: HashMap<String, Vec<TermDoc>>
}

pub struct TermDocComparator {
    pub val: TermDoc,
    pub idx: usize
}

impl Eq for TermDocComparator {}

impl Ord for TermDocComparator {
    fn cmp(&self, other: &Self) -> Ordering {
        other.val.doc_id.cmp(&self.val.doc_id)
    }
}

impl PartialOrd for TermDocComparator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.val.doc_id.cmp(&self.val.doc_id))
    }
}

impl PartialEq for TermDocComparator {
    fn eq(&self, other: &Self) -> bool {
        self.val.doc_id == other.val.doc_id
    }
}

impl WorkerMiner {
    pub fn index_doc (&mut self, doc_id: u32, field_texts: Vec<(String, String)>) {
        for (field_name, field_text) in field_texts {
            let mut field_pos = 0;
            let field_id = self.field_infos.get(&field_name).expect(&format!("Inexistent field: {}", field_name)).id;

            let field_terms = tokenize(&field_text);
            for field_term in field_terms {
                field_pos += 1;

                let term_docs = self.terms.entry(field_term).or_insert_with(Vec::new);

                let term_doc: &mut TermDoc = if let Some(term_doc) = term_docs.last_mut() {
                    if term_doc.doc_id != doc_id {
                        term_docs.push(TermDoc {
                            doc_id,
                            doc_fields: Vec::new()
                        });
                        term_docs.last_mut().unwrap()
                    } else {
                        term_doc
                    }
                } else {
                    term_docs.push(TermDoc {
                        doc_id,
                        doc_fields: Vec::new()
                    });
                    term_docs.last_mut().unwrap()
                };

                let doc_field: &mut DocField = if let Some(doc_field) = term_doc.doc_fields.last_mut() {
                    if doc_field.field_id != field_id {
                        term_doc.doc_fields.push(DocField {
                            field_id,
                            field_positions: Vec::new()
                        });
                        term_doc.doc_fields.last_mut().unwrap()
                    } else {
                        doc_field
                    }
                } else {
                    term_doc.doc_fields.push(DocField {
                        field_id,
                        field_positions: Vec::new()
                    });
                    term_doc.doc_fields.last_mut().unwrap()
                };

                doc_field.field_positions.push(field_pos);
            }
        }
    }
}
