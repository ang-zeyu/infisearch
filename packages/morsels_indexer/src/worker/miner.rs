use morsels_common::tokenize::Tokenizer;
use std::borrow::Cow;
use regex::Regex;
use std::cmp::Ordering;
use std::io::Write;
use std::str;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::FieldInfo;
use crate::FieldInfos;

pub struct DocField {
    pub field_tf: u32,
    pub positions: Vec<u32>,
}

impl Clone for DocField {
    fn clone(&self) -> Self {
        DocField {
            field_tf: self.field_tf,
            positions: self.positions.clone(),
        }
    }
}

impl Default for DocField {
    fn default() -> Self {
        DocField {
            field_tf: 0,
            positions: Vec::new(),
        }
    }
}

pub struct TermDoc {
    pub doc_id: u32,
    pub doc_fields: Vec<DocField>
}

#[derive(Debug)]
pub struct WorkerMinerDocInfo {
    pub doc_id: u32,
    pub field_lengths: Vec<u32>,
    pub field_texts: Vec<u8>,
}

// Intermediate BSBI miner for use in a worker
// Outputs (termID, docID, fieldId, fieldTf, positions ...., fieldId, fieldTf, positions ....) tuples
pub struct WorkerMiner {
    pub field_infos: Arc<FieldInfos>,
    pub with_positions: bool,
    pub terms: FxHashMap<String, Vec<TermDoc>>,
    pub doc_infos: Vec<WorkerMinerDocInfo>,
    pub tokenizer: Arc<dyn Tokenizer + Send + Sync>,
}

pub struct TermDocComparator(pub TermDoc, pub std::vec::IntoIter<TermDoc>);

impl Eq for TermDocComparator {}

impl Ord for TermDocComparator {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.doc_id.cmp(&self.0.doc_id)
    }
}

impl PartialOrd for TermDocComparator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.0.doc_id.cmp(&self.0.doc_id))
    }
}

impl PartialEq for TermDocComparator {
    fn eq(&self, other: &Self) -> bool {
        other.0.doc_id == self.0.doc_id
    }
}

pub struct DocIdAndFieldLengthsComparator(pub WorkerMinerDocInfo, pub std::vec::IntoIter<WorkerMinerDocInfo>);

impl Eq for DocIdAndFieldLengthsComparator {}

impl Ord for DocIdAndFieldLengthsComparator {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.doc_id.cmp(&self.0.doc_id)
    }
}

impl PartialOrd for DocIdAndFieldLengthsComparator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.0.doc_id.cmp(&self.0.doc_id))
    }
}

impl PartialEq for DocIdAndFieldLengthsComparator {
    fn eq(&self, other: &Self) -> bool {
        other.0.doc_id == self.0.doc_id
    }
}

// Adapted from https://lise-henry.github.io/articles/optimising_strings.html
fn find_u8_unsafe_morecap<'a, S: Into<Cow<'a, str>>>(input: S) -> Cow<'a, str> {
    lazy_static! {
        static ref REGEX: Regex = Regex::new(r#"[\n\r\t"\\]"#).unwrap();
    }
    let input = input.into();
    let first = REGEX.find(&input);
    if let Some(first) = first {
        let start = first.start();
        let len = input.len();
        let mut output:Vec<u8> = Vec::with_capacity(len + len/2);
        output.extend_from_slice(input[0..start].as_bytes());
        let rest = input[start..].bytes();
        for c in rest {
            match c {
                b'\n' => output.extend_from_slice(b"\\n"),
                b'\r' => output.extend_from_slice(b"\\r"),
                b'\t' => output.extend_from_slice(b"\\t"),
                b'"' => output.extend_from_slice(b"\\\""),
                b'\\' => output.extend_from_slice(b"\\\\"),
                _ => output.push(c),
            }
        }
        Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
    } else {
        input
    }
}

static NULL_FIELD: FieldInfo = FieldInfo {
    id: 0,
    do_store: false,
    weight: 0.0,
    k: 0.0,
    b: 0.0,
};

impl WorkerMiner {
    pub fn index_doc(&mut self, doc_id: u32, field_texts: Vec<(String, String)>) {
        let mut is_first_stored_field = true;
        
        let mut pos = 0;

        let num_scored_fields = self.field_infos.num_scored_fields;
        let mut field_lengths = vec![0; num_scored_fields];
        let mut field_store_buffered_writer = Vec::with_capacity(
            ((2 + field_texts.iter().fold(0, |acc, b| acc + 7 + b.1.len())) as f32 * 1.1) as usize
        );
        field_store_buffered_writer.write_all("[".as_bytes()).unwrap();

        for (field_name, field_text) in field_texts {
            let field_info = self.field_infos.field_infos_map.get(&field_name).unwrap_or(&NULL_FIELD);
            let field_id = field_info.id;

            // Store raw text
            if field_info.do_store {
                if !is_first_stored_field {
                    field_store_buffered_writer.write_all(b",").unwrap();
                } else {
                    is_first_stored_field = false;
                }
                field_store_buffered_writer.write_all(b"[").unwrap();
                field_store_buffered_writer.write_all(field_id.to_string().as_bytes()).unwrap();
                field_store_buffered_writer.write_all(b",\"").unwrap();
                field_store_buffered_writer.write_all(find_u8_unsafe_morecap(&field_text).as_bytes()).unwrap();
                field_store_buffered_writer.write_all(b"\"]").unwrap();
            }

            if field_info.weight == 0.0 {
                continue;
            }

            let field_terms = self.tokenizer.tokenize(field_text);

            *field_lengths.get_mut(field_id as usize).unwrap() += field_terms.len() as u32;

            for field_term in field_terms {
                pos += 1;

                let term_docs = self.terms.entry(field_term)
                    .or_insert_with(|| vec![TermDoc {
                        doc_id,
                        doc_fields: vec![DocField::default(); num_scored_fields]
                    }]);

                let mut term_doc = term_docs.last_mut().unwrap();
                if term_doc.doc_id != doc_id {
                    term_docs.push(TermDoc {
                        doc_id,
                        doc_fields: vec![DocField::default(); num_scored_fields]
                    });
                    term_doc = term_docs.last_mut().unwrap();
                }

                let doc_field = term_doc.doc_fields.get_mut(field_id as usize).unwrap();
                doc_field.field_tf += 1;
                if self.with_positions {
                    doc_field.positions.push(pos);
                }
            }

            pos += 120; // to "split up zones"
        }

        field_store_buffered_writer.write_all(b"]").unwrap();
        field_store_buffered_writer.flush().unwrap();
        self.doc_infos.push(WorkerMinerDocInfo {
            doc_id,
            field_lengths,
            field_texts: field_store_buffered_writer,
        });
    }
}
