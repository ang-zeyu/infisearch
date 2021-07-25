use std::cmp::Ordering;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::JsValue;
use web_sys::Response;

use librarian_common::tokenize::TermInfo;
use crate::utils::varint::decode_var_int;

pub struct DocField {
    pub field_id: u8,
    pub field_positions: Vec<u32>,
}

impl Clone for DocField {
    fn clone(&self) -> Self {
        DocField {
            field_id: self.field_id,
            field_positions: self.field_positions.clone(),
        }
    }
}

pub struct TermDoc {
    pub doc_id: u32,
    pub fields: Vec<DocField>,
}

impl Clone for TermDoc {
    fn clone(&self) -> Self {
        TermDoc {
            doc_id: self.doc_id,
            fields: self.fields.clone(),
        }
    }
}

pub struct PlIterator<'a> {
    pub td: Option<&'a TermDoc>,
    pub pl: &'a PostingsList,
    idx: usize,
    pub original_idx: u8,
}

impl<'a> PlIterator<'a> {
    pub fn next(&mut self) -> Option<&'a TermDoc> {
        self.idx += 1;
        self.td = self.pl.term_docs.get(self.idx);
        self.td
    }

    pub fn peek_prev(&self) -> Option<&'a TermDoc> {
        self.pl.term_docs.get(self.idx - 1)
    }
}

// Order by term, then block number
impl<'a> Eq for PlIterator<'a> {}

impl<'a> PartialEq for PlIterator<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<'a> Ord for PlIterator<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.td.unwrap().doc_id.cmp(&other.td.unwrap().doc_id)
    }
}

impl<'a> PartialOrd for PlIterator<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Option::from(self.td.unwrap().doc_id.cmp(&other.td.unwrap().doc_id))
    }
}

pub struct PostingsList {
    pub term_docs: Vec<TermDoc>,
    pub weight: f32,
    pub idf: f64,
    pub include_in_proximity_ranking: bool,
    // For postings lists representing raw terms
    pub term: Option<String>,
    pub term_info: Option<Rc<TermInfo>>,
}

impl PostingsList {
    pub fn get_it(&self, original_idx: u8) -> PlIterator {
        PlIterator {
            td: self.term_docs.get(0),
            pl: self,
            idx: 0,
            original_idx,
        }
    }

    // Used for "processed" (e.g. phrase, bracket, AND) postings lists
    pub fn calc_pseudo_idf(&mut self, num_docs: u32) {
        self.idf = (1.0 + (num_docs as f64 - self.term_docs.len() as f64 + 0.5) / (self.term_docs.len() as f64 + 0.5)).ln()
    }

    pub fn merge_term_docs(term_doc_1: &TermDoc, term_doc_2: &TermDoc) -> TermDoc {
        let max_fields_length = std::cmp::max(term_doc_1.fields.len(), term_doc_2.fields.len());

        let mut td = TermDoc {
            doc_id: term_doc_1.doc_id,
            fields: Vec::with_capacity(max_fields_length),
        };

        for field_id in 0..max_fields_length {
            let term_doc_1_field_opt = term_doc_1.fields.get(field_id);
            let term_doc_2_field_opt = term_doc_2.fields.get(field_id);

            if term_doc_1_field_opt.is_some() && term_doc_2_field_opt.is_some() {
                let term_doc_1_field = term_doc_1_field_opt.unwrap();
                let term_doc_2_field = term_doc_2_field_opt.unwrap();
                let mut doc_field = DocField {
                    field_id: field_id as u8,
                    field_positions: Vec::new(),
                };

                let mut pos2_idx = 0;
                for pos1_idx in 0..term_doc_1_field.field_positions.len() {
                    while pos2_idx < term_doc_2_field.field_positions.len()
                        && term_doc_2_field.field_positions[pos2_idx] < term_doc_1_field.field_positions[pos1_idx] {
                        doc_field.field_positions.push(term_doc_2_field.field_positions[pos2_idx]);
                        pos2_idx += 1;
                    }

                    if pos2_idx < term_doc_2_field.field_positions.len()
                        && term_doc_2_field.field_positions[pos2_idx] == term_doc_1_field.field_positions[pos1_idx] {
                        pos2_idx += 1;
                    }

                    doc_field.field_positions.push(term_doc_1_field.field_positions[pos1_idx]);
                }

                while pos2_idx < term_doc_2_field.field_positions.len() {
                    doc_field.field_positions.push(term_doc_2_field.field_positions[pos2_idx]);
                    pos2_idx += 1;
                }

                td.fields.push(doc_field);
            } else if let Option::Some(term_doc_1_field) = term_doc_1_field_opt {
                td.fields.push(DocField {
                    field_id: field_id as u8,
                    field_positions: term_doc_1_field.field_positions.clone(),
                });
            } else if let Option::Some(term_doc_2_field) = term_doc_2_field_opt {
                td.fields.push(DocField {
                    field_id: field_id as u8,
                    field_positions: term_doc_2_field.field_positions.clone(),
                });
            }
        }

        td
    }

    pub async fn fetch_term(&mut self, base_url: &str, window: &web_sys::Window, num_scored_fields: usize) -> Result<(), JsValue> {
        if let Option::None = self.term_info {
            return Ok(());
        }

        let term_info = self.term_info.as_ref().unwrap();
        
        let pl_resp_value = JsFuture::from(
            window.fetch_with_str(&(base_url.to_owned() + "/pl_" + &term_info.postings_file_name.to_string()))
        ).await?;
        let pl_resp: Response = pl_resp_value.dyn_into().unwrap();
        let pl_array_buffer = JsFuture::from(pl_resp.array_buffer()?).await?;
        let pl_vec = js_sys::Uint8Array::new(&pl_array_buffer).to_vec();

        let mut prev_doc_id = 0;
        let mut pos: usize = term_info.postings_file_offset as usize;
        for _i in 0..term_info.doc_freq {
            let docfreq_and_len = decode_var_int(&pl_vec[pos..]);
            pos += docfreq_and_len.1;

            let mut term_doc = TermDoc {
                doc_id: prev_doc_id + docfreq_and_len.0,
                fields: Vec::with_capacity(num_scored_fields),
            };
            prev_doc_id = term_doc.doc_id;

            let mut is_last: u8 = 0;
            while is_last == 0 {
                let next_int = pl_vec[pos];
                pos += 1;

                let field_id = next_int & 0x7f;
                is_last = next_int & 0x80;

                let field_tf_val_and_length = decode_var_int(&pl_vec[pos..]);
                pos += field_tf_val_and_length.1;

                let mut field_positions = Vec::with_capacity(field_tf_val_and_length.0 as usize);
                let mut prev_pos = 0;
                for _j in 0..field_tf_val_and_length.0 {
                    let posgap_val_and_length = decode_var_int(&pl_vec[pos..]);
                    pos += posgap_val_and_length.1;

                    prev_pos += posgap_val_and_length.0;
                    field_positions.push(prev_pos);
                }

                for field_id_before in term_doc.fields.len() as u8..field_id {
                    term_doc.fields.push(DocField {
                        field_id: field_id_before,
                        field_positions: Vec::new(),
                    });
                }
                
                term_doc.fields.push(DocField {
                    field_id,
                    field_positions,
                });
            }

            self.term_docs.push(term_doc);
        }

        Ok(())
    }
}
