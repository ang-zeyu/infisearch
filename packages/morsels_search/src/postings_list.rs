use std::cmp::Ordering;

use byteorder::{ByteOrder, LittleEndian};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use crate::postings_list_file_cache::PostingsListFileCache;
use morsels_common::tokenize::TermInfo;
use morsels_common::utils::idf::get_idf;
use morsels_common::utils::varint::decode_var_int;

#[cfg_attr(test, derive(Debug))]
pub struct DocField {
    pub field_tf: f32,
    pub field_positions: Vec<u32>,
}

#[cfg(test)]
impl Eq for DocField {}

#[cfg(test)]
impl PartialEq for DocField {
    fn eq(&self, other: &Self) -> bool {
        self.field_tf as u32 == other.field_tf as u32 && self.field_positions == other.field_positions
    }
}

impl Clone for DocField {
    fn clone(&self) -> Self {
        DocField { field_tf: self.field_tf, field_positions: self.field_positions.clone() }
    }
}

impl Default for DocField {
    fn default() -> Self {
        DocField { field_tf: 0.0, field_positions: Vec::new() }
    }
}

#[cfg_attr(test, derive(Eq, PartialEq, Debug))]
pub struct TermDoc {
    pub doc_id: u32,
    pub fields: Vec<DocField>,
}

impl TermDoc {
    pub fn to_owned(&self) -> Self {
        TermDoc { doc_id: self.doc_id, fields: self.fields.clone() }
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

/*
 Assumes term docs to be is_some()!
 (iterator still has some docs left)
 */
impl<'a> Ord for PlIterator<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.td.unwrap().doc_id.cmp(&other.td.unwrap().doc_id)
    }
}

impl<'a> PartialOrd for PlIterator<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.td.unwrap().doc_id.cmp(&other.td.unwrap().doc_id))
    }
}

pub struct PostingsList {
    pub term_docs: Vec<TermDoc>,
    pub weight: f32,
    pub idf: f64,
    pub include_in_proximity_ranking: bool,
    // For postings lists representing raw terms
    pub term: Option<String>,
    pub term_info: Option<TermInfo>,
    pub max_term_score: f32,
}

#[cfg(test)]
impl Eq for PostingsList {}

#[cfg(test)]
impl PartialEq for PostingsList {
    fn eq(&self, other: &Self) -> bool {
        self.term_docs == other.term_docs
    }
}

#[cfg(test)]
impl std::fmt::Debug for PostingsList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostingsList").field("term_docs", &self.term_docs).finish()
    }
}

impl PostingsList {
    pub fn get_it(&self, original_idx: u8) -> PlIterator {
        PlIterator { td: self.term_docs.get(0), pl: self, idx: 0, original_idx }
    }

    // Used for "processed" (e.g. phrase, bracket, AND) postings lists
    pub fn calc_pseudo_idf(&mut self, num_docs: u32) {
        self.idf = get_idf(num_docs as f64, self.term_docs.len() as f64);
    }

    pub fn merge_term_docs(term_doc_1: &TermDoc, term_doc_2: &TermDoc) -> TermDoc {
        let max_fields_length = std::cmp::max(term_doc_1.fields.len(), term_doc_2.fields.len());

        let mut td = TermDoc { doc_id: term_doc_1.doc_id, fields: Vec::with_capacity(max_fields_length) };

        for field_id in 0..max_fields_length {
            let term_doc_1_field_opt = term_doc_1.fields.get(field_id);
            let term_doc_2_field_opt = term_doc_2.fields.get(field_id);

            if let Some(term_doc_1_field) = term_doc_1_field_opt {
                if let Some(term_doc_2_field) = term_doc_2_field_opt {
                    let mut doc_field = DocField {
                        field_tf: term_doc_1_field.field_tf + term_doc_2_field.field_tf,
                        field_positions: Vec::new(),
                    };

                    let mut pos2_idx = 0;
                    for pos1_idx in 0..term_doc_1_field.field_positions.len() {
                        while pos2_idx < term_doc_2_field.field_positions.len()
                            && term_doc_2_field.field_positions[pos2_idx]
                                < term_doc_1_field.field_positions[pos1_idx]
                        {
                            doc_field.field_positions.push(term_doc_2_field.field_positions[pos2_idx]);
                            pos2_idx += 1;
                        }

                        if pos2_idx < term_doc_2_field.field_positions.len()
                            && term_doc_2_field.field_positions[pos2_idx]
                                == term_doc_1_field.field_positions[pos1_idx]
                        {
                            pos2_idx += 1;
                        }

                        doc_field.field_positions.push(term_doc_1_field.field_positions[pos1_idx]);
                    }

                    while pos2_idx < term_doc_2_field.field_positions.len() {
                        doc_field.field_positions.push(term_doc_2_field.field_positions[pos2_idx]);
                        pos2_idx += 1;
                    }

                    td.fields.push(doc_field);
                } else {
                    td.fields.push(term_doc_1_field.clone());
                }
            } else if let Some(term_doc_2_field) = term_doc_2_field_opt {
                td.fields.push(term_doc_2_field.clone());
            }
        }

        td
    }

    #[inline]
    pub async fn fetch_pl_to_vec(
        window: &web_sys::Window,
        base_url: &str,
        pl_num: u32,
        num_pls_per_dir: u32,
    ) -> Result<Vec<u8>, JsValue> {
        let pl_resp_value = JsFuture::from(window.fetch_with_str(
            &(base_url.to_owned()
                + "pl_"
                + &(pl_num / num_pls_per_dir).to_string()[..]
                + "/pl_"
                + &pl_num.to_string()[..]),
        ))
        .await?;
        let pl_resp: Response = pl_resp_value.dyn_into().unwrap();
        let pl_array_buffer = JsFuture::from(pl_resp.array_buffer()?).await?;
        Ok(js_sys::Uint8Array::new(&pl_array_buffer).to_vec())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn fetch_term(
        &mut self,
        base_url: &str,
        pl_file_cache: &PostingsListFileCache,
        invalidation_vector: &[u8],
        window: &web_sys::Window,
        num_scored_fields: usize,
        num_pls_per_dir: u32,
        with_positions: bool,
    ) -> Result<(), JsValue> {
        if self.term_info.is_none() {
            return Ok(());
        }

        let term_info = self.term_info.as_ref().unwrap();

        let fetched_pl;
        let pl_vec = if let Some(pl_vec) = pl_file_cache.get(term_info.postings_file_name) {
            pl_vec
        } else {
            fetched_pl = PostingsList::fetch_pl_to_vec(
                window,
                base_url,
                term_info.postings_file_name,
                num_pls_per_dir,
            )
            .await?;
            &fetched_pl
        };

        let mut pos = term_info.postings_file_offset as usize;

        let mut prev_doc_id = 0;
        for _i in 0..term_info.doc_freq {
            let docfreq = decode_var_int(&pl_vec, &mut pos);

            let mut term_doc = TermDoc {
                doc_id: prev_doc_id + docfreq,
                fields: Vec::with_capacity(num_scored_fields),
            };
            prev_doc_id = term_doc.doc_id;

            let mut is_last: u8 = 0;
            while is_last == 0 {
                let next_int = pl_vec[pos];
                pos += 1;

                let field_id = next_int & 0x7f;
                is_last = next_int & 0x80;

                let field_tf = decode_var_int(&pl_vec, &mut pos);

                let field_positions = if with_positions {
                    let mut field_positions = Vec::with_capacity(field_tf as usize);

                    let mut prev_pos = 0;
                    for _j in 0..field_tf {
                        prev_pos += decode_var_int(&pl_vec, &mut pos);
                        field_positions.push(prev_pos);
                    }

                    field_positions
                } else {
                    Vec::new()
                };

                for _field_id_before in term_doc.fields.len() as u8..field_id {
                    term_doc.fields.push(DocField::default());
                }

                term_doc.fields.push(DocField { field_tf: field_tf as f32, field_positions });
            }

            if !morsels_common::bitmap::check(invalidation_vector, prev_doc_id as usize) {
                self.term_docs.push(term_doc);
            }
        }

        self.max_term_score = LittleEndian::read_f32(&pl_vec[pos..]) * self.weight;

        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use std::rc::Rc;

    use pretty_assertions::assert_eq;

    use super::{DocField, PostingsList, TermDoc};

    // Takes a vector of "TermDoc", containing a vector of "fields", containing a tuple of (field_tf, vector of field positions)
    // E.g. a TermDoc containing 2 fields of term frequency 2 and 1: [ [2,[1,2]], [1,[120]] ]
    pub fn to_pl(text: &str) -> PostingsList {
        let vec: Vec<Option<Vec<(f32, Vec<u32>)>>> = serde_json::from_str(&format!("[{}]", text)).unwrap();

        let term_docs: Vec<TermDoc> = vec
            .into_iter()
            .enumerate()
            .filter(|doc_fields| doc_fields.1.is_some())
            .map(|doc_fields| (doc_fields.0, doc_fields.1.unwrap()))
            .map(|(doc_id, doc_fields)| vec_to_term_doc(doc_id as u32, doc_fields))
            .collect();

        PostingsList {
            term_docs,
            weight: 1.0,
            idf: 1.0,
            include_in_proximity_ranking: true,
            term: None,
            term_info: None,
            max_term_score: 0.0,
        }
    }

    fn vec_to_term_doc(doc_id: u32, doc_fields: Vec<(f32, Vec<u32>)>) -> TermDoc {
        let fields: Vec<DocField> = doc_fields
            .into_iter()
            .map(|(field_tf, field_positions)| DocField { field_tf, field_positions })
            .collect();

        TermDoc { doc_id, fields }
    }

    pub fn to_pl_rc(text: &str) -> Rc<PostingsList> {
        Rc::new(to_pl(text))
    }

    fn to_term_doc(text: &str) -> TermDoc {
        let doc_fields: Vec<(f32, Vec<u32>)> = serde_json::from_str(text).unwrap();
        vec_to_term_doc(0, doc_fields)
    }

    #[test]
    fn test_term_doc_merge() {
        assert_eq!(
            PostingsList::merge_term_docs(&to_term_doc("[ [2,[1,2]] ]"), &to_term_doc("[ [1,[120]] ]"),),
            to_term_doc("[ [3,[1,2,120]] ]"),
        );

        assert_eq!(
            PostingsList::merge_term_docs(
                &to_term_doc("[ [2,[1,2]] ]"),
                &to_term_doc("[ [0,[]], [1,[120]] ]"),
            ),
            to_term_doc("[ [2,[1,2]], [1,[120]] ]"),
        );

        assert_eq!(
            PostingsList::merge_term_docs(
                &to_term_doc("[ [2,[1,2]], [1,[120]] ]"),
                &to_term_doc("[ [2,[1,2]] ]"),
            ),
            to_term_doc("[ [4,[1,2]], [1,[120]] ]"),
        );

        assert_eq!(
            PostingsList::merge_term_docs(
                &to_term_doc("[ [2,[1,2]], [1,[120]] ]"),
                &to_term_doc("[ [2,[1,2]], [1,[121]] ]"),
            ),
            to_term_doc("[ [4,[1,2]], [2,[120,121]] ]"),
        );
    }
}
