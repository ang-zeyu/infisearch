use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crate::Dictionary;
use crate::fieldinfo::FieldInfo;
use crate::tokenize::english::tokenize;

struct DocField {
    field_id: u8,
    field_tf: u32,
    field_positions: Vec<u32>
}

pub struct TermDoc {
    doc_id: u32,
    doc_fields: Vec<DocField>
}

// Intermediate BSBI miner for use in a worker
// Outputs (termID, docID, fieldId, fieldTf, positions ...., fieldId, fieldTf, positions ....) tuples
pub struct WorkerMiner {
    pub field_infos: Arc<HashMap<String, FieldInfo>>,

    pub terms: HashMap<u32, Vec<TermDoc>>
}

impl WorkerMiner {
    pub fn combine_and_sort(worker_miners: Vec<WorkerMiner>) -> Vec<(u32, Vec<TermDoc>)> {
        let mut combined_terms: HashMap<u32, Vec<TermDoc>> = HashMap::new();

        // Combine
        for worker_miner in worker_miners {
            for (worker_term, worker_term_docs) in worker_miner.terms {
                let combined_term_docs = combined_terms.entry(worker_term).or_insert(Vec::new());
                combined_term_docs.extend(worker_term_docs);
            }
        }

        // Sort
        let mut sorted_entries: Vec<(u32, Vec<TermDoc>)> = combined_terms.into_iter().collect();
        sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));

        sorted_entries
    }

    pub fn write_bsbi_block(bsbi_block: Vec<(u32, Vec<TermDoc>)>, output_folder_path: &Path, bsbi_block_number: u32) {
        let output_file_path = Path::new(output_folder_path).join(format!("bsbi_block_{}", bsbi_block_number));

        println!("Writing bsbi block {} to {}, num terms {}", bsbi_block_number, output_file_path.to_str().unwrap(), bsbi_block.len());

        let f = File::create(output_file_path).expect("Failed to open dictionary string for writing.");
        let mut buffered_writer = BufWriter::new(f);
        
        for (term_id, term_docs) in bsbi_block {
            // println!("Writing {}", term);

            buffered_writer.write_all(&term_id.to_le_bytes()).unwrap();
            for term_doc in term_docs {
                buffered_writer.write_all(&term_doc.doc_id.to_le_bytes()).unwrap();

                let num_fields: u8 = term_doc.doc_fields.len().try_into().unwrap();
                buffered_writer.write_all(&[num_fields]).unwrap();

                for doc_field in term_doc.doc_fields {
                    buffered_writer.write_all(&[doc_field.field_id]).unwrap();
                    buffered_writer.write_all(&doc_field.field_tf.to_le_bytes()).unwrap();

                    for pos in doc_field.field_positions {
                        buffered_writer.write_all(&pos.to_le_bytes()).unwrap();
                    }
                }
            }
        }

        buffered_writer.flush().unwrap();
    }

    pub fn index_doc (&mut self, doc_id: u32, field_texts: Vec<(String, String)>, dictionary: &Arc<Dictionary>) {
        for (field_name, field_text) in field_texts {
            let mut field_pos = 0;
            let field_id = self.field_infos.get(&field_name).expect(&format!("Inexistent field: {}", field_name)).id;

            let field_terms = tokenize(&field_text);
            for field_term in field_terms {
                field_pos += 1;

                let term_id = 1;

                let term_docs = self.terms.entry(term_id).or_insert(Vec::new());

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
                            field_tf: 0,
                            field_positions: Vec::new()
                        });
                        term_doc.doc_fields.last_mut().unwrap()
                    } else {
                        doc_field
                    }
                } else {
                    term_doc.doc_fields.push(DocField {
                        field_id,
                        field_tf: 0,
                        field_positions: Vec::new()
                    });
                    term_doc.doc_fields.last_mut().unwrap()
                };

                doc_field.field_tf += 1;
                doc_field.field_positions.push(field_pos);
            }
        }
    }
}
