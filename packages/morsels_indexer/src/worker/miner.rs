use std::borrow::Cow;
use std::cmp::Ordering;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::str;
use std::sync::Arc;

use log::warn;
use path_absolutize::Absolutize;
use regex::Regex;
use rustc_hash::FxHashMap;

use morsels_common::tokenize::IndexerTokenizer;

use crate::fieldinfo::{ADD_FILES_FIELD, FieldInfo, FieldInfos};
use crate::loader::LoaderBoxed;
use crate::i_debug;

const ZONE_SEPARATION: u32 = 10;

#[derive(Default)]
pub struct DocField {
    pub field_tf: u32,
    pub positions: Vec<u32>,
}

impl Clone for DocField {
    fn clone(&self) -> Self {
        DocField { field_tf: self.field_tf, positions: self.positions.clone() }
    }
}

pub struct TermDoc {
    pub doc_id: u32,
    pub doc_fields: Vec<DocField>,
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
    pub tokenizer: Arc<dyn IndexerTokenizer + Send + Sync>,

    input_folder: PathBuf,
    loaders: Arc<Vec<LoaderBoxed>>,
    secondary_inv_mappings: FxHashMap<u32, Vec<String>>,

    #[cfg(debug_assertions)]
    pub id: usize,
    #[cfg(debug_assertions)]
    pub total_terms: u32,
    #[cfg(debug_assertions)]
    pub total_len: u64,
    #[cfg(debug_assertions)]
    pub total_pos: u64,
}

pub struct WorkerBlockIndexResults {
    pub terms: FxHashMap<String, Vec<TermDoc>>,
    pub doc_infos: Vec<WorkerMinerDocInfo>,
    pub secondary_inv_mappings: FxHashMap<u32, Vec<String>>,
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
        static ref REGEX: Regex = Regex::new(r#"[\n\r\t"\\\x08\x0c]"#).unwrap();
    }
    let input = input.into();
    let first = REGEX.find(&input);
    if let Some(first) = first {
        let start = first.start();
        let len = input.len();
        let mut output: Vec<u8> = Vec::with_capacity(len + len / 2);
        output.extend_from_slice(input[0..start].as_bytes());
        let rest = input[start..].bytes();
        for c in rest {
            match c {
                8 => output.extend_from_slice(b"\\b"),
                12 => output.extend_from_slice(b"\\f"),
                b'\n' => output.extend_from_slice(b"\\n"),
                b'\r' => output.extend_from_slice(b"\\r"),
                b'\t' => output.extend_from_slice(b"\\t"),
                b'"' => output.extend_from_slice(b"\\\""),
                b'\\' => output.extend_from_slice(b"\\\\"),
                // All other control characters should use unicode escape sequences
                0..=31 => output.extend_from_slice(format!("\\u00{:02X?}", c).as_bytes()),
                _ => output.push(c),
            }
        }
        Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
    } else {
        input
    }
}

const NULL_FIELD: FieldInfo = FieldInfo { id: 0, do_store: false, weight: 0.0, k: 0.0, b: 0.0 };

impl WorkerMiner {
    pub fn new(
        field_infos: &Arc<FieldInfos>,
        with_positions: bool,
        expected_num_docs_per_reset: usize,
        tokenizer: &Arc<dyn IndexerTokenizer + Send + Sync>,
        input_folder: PathBuf,
        loaders: &Arc<Vec<LoaderBoxed>>,
        #[cfg(debug_assertions)]
        id: usize,
    ) -> Self {
        WorkerMiner {
            field_infos: Arc::clone(field_infos),
            with_positions,
            terms: FxHashMap::default(),
            doc_infos: Vec::with_capacity(expected_num_docs_per_reset),
            tokenizer: Arc::clone(tokenizer),
            input_folder,
            loaders: Arc::clone(loaders),
            secondary_inv_mappings: FxHashMap::default(),

            #[cfg(debug_assertions)]
            id,
            #[cfg(debug_assertions)]
            total_terms: 0,
            #[cfg(debug_assertions)]
            total_len: 0,
            #[cfg(debug_assertions)]
            total_pos: 0,
        }
    }

    pub fn get_results(&mut self) -> WorkerBlockIndexResults {
        let old_doc_infos_capacity = self.doc_infos.capacity();

        #[cfg(debug_assertions)]
        {
            let num_docs = self.doc_infos.len() as u64;
            let average_pos = if num_docs == 0 {
                0
            } else {
                self.total_pos / num_docs
            };
            i_debug!(
                "Worker {}, num_docs {}, total_len {}, total_terms {}, average_pos {}",
                self.id, num_docs, self.total_len, self.total_terms, average_pos,
            );
            self.total_len = 0;
            self.total_terms = 0;
            self.total_pos = 0;
        }

        WorkerBlockIndexResults {
            terms: std::mem::take(&mut self.terms),
            doc_infos: std::mem::replace(&mut self.doc_infos, Vec::with_capacity(old_doc_infos_capacity)),
            secondary_inv_mappings: std::mem::take(&mut self.secondary_inv_mappings),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_other_file(
        &mut self,
        add_files_field_text: String,
        original_absolute_path: &Path,
        is_first_stored_field: &mut bool,
        field_store_buffered_writer: &mut Vec<u8>,
        field_lengths: &mut Vec<u32>,
        doc_id: u32,
        num_scored_fields: usize,
        pos: &mut u32,
    ) {
        let path = PathBuf::from(add_files_field_text);
        let (absolute_path, relative_path) = if path.is_absolute() {
            let absolute_path =  if let Ok(path) = path.absolutize() {
                path.to_path_buf()
            } else {
                warn_missing_other_file(&path, original_absolute_path);
                return;
            };

            let relative_path = pathdiff::diff_paths(&absolute_path, &self.input_folder)
                .expect("Relative path construction failed");

            (absolute_path, relative_path)
        } else {
            let absolute_path = if let Ok(path) = original_absolute_path.with_file_name(&path).absolutize() {
                path.to_path_buf()
            } else {
                warn_missing_other_file(&path, original_absolute_path);
                return;
            };

            let relative_path = pathdiff::diff_paths(&absolute_path, &self.input_folder)
                .expect("Relative path construction failed");

            (absolute_path, relative_path)
        };

        i_debug!(
            "Linking in\n  (absolute) {}\n  (relative) {}\n  (from)     {}\n",
            absolute_path.to_string_lossy(),
            relative_path.to_string_lossy(),
            original_absolute_path.to_string_lossy(),
        );

        self.secondary_inv_mappings.entry(doc_id)
            .or_insert_with(Vec::new)
            .push(if let Some(relative_path) = relative_path.to_str() {
                relative_path.to_owned()
            } else {
                relative_path.to_string_lossy().into_owned()
            });

        if !absolute_path.exists() {
            warn_missing_other_file(&absolute_path, original_absolute_path);
            return;
        }

        for loader in Arc::clone(&self.loaders).iter() {
            if let Some(loader_results) = loader.try_index_file(&absolute_path, &relative_path)
            {
                for loader_result in loader_results {
                    let (field_texts, path) = loader_result.get_field_texts_and_path();
                    self.process_field_texts(
                        field_texts,
                        path,
                        is_first_stored_field,
                        field_store_buffered_writer,
                        field_lengths,
                        doc_id,
                        num_scored_fields,
                        pos,
                    );
                }
        
                break;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_field_texts(
        &mut self,
        field_texts: Vec<(String, String)>,
        original_absolute_path: PathBuf,
        is_first_stored_field: &mut bool,
        field_store_buffered_writer: &mut Vec<u8>,
        field_lengths: &mut Vec<u32>,
        doc_id: u32,
        num_scored_fields: usize,
        pos: &mut u32,
    ) {
        for (field_name, mut field_text) in field_texts {
            if field_name == ADD_FILES_FIELD {
                self.add_other_file(
                    field_text,
                    &original_absolute_path,
                    is_first_stored_field,
                    field_store_buffered_writer,
                    field_lengths,
                    doc_id,
                    num_scored_fields,
                    pos,
                );
                continue;
            }

            let field_info = self.field_infos.field_infos_map.get(&field_name).unwrap_or(&NULL_FIELD);
            let field_id = field_info.id;

            // ----------------------------------------------
            // Json field stores
            if field_info.do_store {
                if !(*is_first_stored_field) {
                    field_store_buffered_writer.write_all(b",").unwrap();
                } else {
                    *is_first_stored_field = false;
                }
                field_store_buffered_writer.write_all(b"[").unwrap();
                field_store_buffered_writer.write_all(field_id.to_string().as_bytes()).unwrap();
                field_store_buffered_writer.write_all(b",\"").unwrap();
                field_store_buffered_writer
                    .write_all(find_u8_unsafe_morecap(&field_text).as_bytes())
                    .unwrap();
                field_store_buffered_writer.write_all(b"\"]").unwrap();
            }
            // ----------------------------------------------

            if field_info.weight == 0.0 {
                continue;
            }

            #[cfg(debug_assertions)]
            {
                self.total_len += field_text.len() as u64;
            }

            let terms = self.tokenizer.tokenize(&mut field_text);
            let field_lengths = field_lengths.get_mut(field_id as usize).unwrap();

            for term in terms {
                if let Some(term) = term {
                    *field_lengths += 1;

                    #[cfg(debug_assertions)]
                    {
                        self.total_terms += 1;
                    }

                    let term_docs = if let Some(existing) = self.terms.get_mut(&term[..]) {
                        existing
                    } else {
                        self.terms.entry(term.into_owned()).or_insert_with(|| vec![TermDoc {
                            doc_id,
                            doc_fields: vec![DocField::default(); num_scored_fields],
                        }])
                    };

                    let mut term_doc = term_docs.last_mut().unwrap();
                    if term_doc.doc_id != doc_id {
                        term_docs.push(TermDoc {
                            doc_id,
                            doc_fields: vec![DocField::default(); num_scored_fields],
                        });
                        term_doc = term_docs.last_mut().unwrap();
                    }

                    let doc_field = term_doc.doc_fields.get_mut(field_id as usize).unwrap();
                    doc_field.field_tf += 1;
                    if self.with_positions {
                        doc_field.positions.push(*pos);
                    }
                }

                *pos += 1;
            }

            // To split up "zones" positionally (separate field tuples)
            // TODO consider making this smarter / configurable
            *pos += ZONE_SEPARATION;
        }
    }

    pub fn index_doc(&mut self, doc_id: u32, field_texts: Vec<(String, String)>, original_absolute_path: PathBuf) {
        let mut is_first_stored_field = true;

        let mut pos = 0;

        let num_scored_fields = self.field_infos.num_scored_fields;
        let mut field_lengths = vec![0; num_scored_fields];
        let mut field_store_buffered_writer = Vec::with_capacity(
            ((2 + field_texts.iter().fold(0, |acc, b| acc + 7 + b.1.len())) as f32 * 1.1) as usize,
        );
        field_store_buffered_writer.write_all("[".as_bytes()).unwrap();

        self.process_field_texts(
            field_texts,
            original_absolute_path,
            &mut is_first_stored_field,
            &mut field_store_buffered_writer,
            &mut field_lengths,
            doc_id,
            num_scored_fields,
            &mut pos,
        );

        #[cfg(debug_assertions)]
        {
            self.total_pos += pos as u64;
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

fn warn_missing_other_file(absolute_path: &Path, original_absolute_path: &Path) {
    warn!(
        "Other file {} linked from {} does not exist! Skipping",
        absolute_path.to_string_lossy(),
        original_absolute_path.to_string_lossy(),
    );
}
