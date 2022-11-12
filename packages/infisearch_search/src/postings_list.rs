use std::rc::Rc;

use infisearch_common::dictionary::TermInfo;
use infisearch_common::packed_var_int::read_bits_from;
use infisearch_common::postings_list::{
    LAST_FIELD_MASK, SHORT_FORM_MASK,
    MIN_CHUNK_SIZE, CHUNK_SIZE,
};
use infisearch_common::utils::idf::get_idf;
use infisearch_common::utils::varint::decode_var_int;

pub fn get_postings_list<'a, 'b>(
    term: &'b str,
    postings_lists: &'a Vec<PostingsList>,
) -> Option<&'a PostingsList> {
    postings_lists.iter().find(|pl| unsafe { pl.term.as_ref().unwrap_unchecked() } == term)
}

pub fn get_postings_list_rc<'a, 'b>(
    term: &'b str,
    postings_lists: &'a Vec<Rc<PostingsList>>,
) -> Option<&'a Rc<PostingsList>> {
    postings_lists.iter().find(|pl| unsafe { pl.term.as_ref().unwrap_unchecked() } == term)
}

#[cfg_attr(test, derive(Debug))]
pub struct Field {
    pub field_tf: f32,
    pub field_positions: Vec<u32>,
}

#[cfg(test)]
impl Eq for Field {}

#[cfg(test)]
impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.field_tf as u32 == other.field_tf as u32 && self.field_positions == other.field_positions
    }
}

impl Clone for Field {
    fn clone(&self) -> Self {
        Field { field_tf: self.field_tf, field_positions: self.field_positions.clone() }
    }
}

impl Default for Field {
    fn default() -> Self {
        Field { field_tf: 0.0, field_positions: Vec::new() }
    }
}

#[cfg_attr(test, derive(Debug))]
pub struct Doc {
    pub doc_id: u32,
    pub fields: Vec<Field>,
    pub score: f32, 
}

#[cfg(test)]
impl PartialEq for Doc {
    fn eq(&self, other: &Self) -> bool {
        self.doc_id == other.doc_id && self.fields == other.fields
    }

    fn ne(&self, other: &Self) -> bool {
        self.doc_id != other.doc_id || self.fields != other.fields
    }
}

impl Doc {
    pub fn to_owned(&self) -> Self {
        Doc { doc_id: self.doc_id, fields: self.fields.clone(), score: self.score }
    }
}

/// Dosen't actually implement the iterator interfaces
/// 
/// Facilitates repeated access to the current value (td)
/// and previous value (peek_prev).
pub struct PlIterator<'a> {
    pub prev_td: Option<&'a Doc>,
    pub td: Option<&'a Doc>,
    pub pl: &'a PostingsList,
    idx: usize,
    pub weight: f32,
    pub include_in_proximity_ranking: bool,
    pub is_mandatory: bool,
    pub is_subtracted: bool,
    pub is_inverted: bool,
}

impl<'a> PlIterator<'a> {
    pub fn next(&mut self) -> Option<&'a Doc> {
        self.idx += 1;
        self.prev_td = self.td;
        self.td = self.pl.term_docs.get(self.idx);
        self.td
    }
}

pub struct PostingsList {
    pub term_docs: Vec<Doc>,
    pub idf: f32,
    // For postings lists representing raw terms
    pub term: Option<String>,
    pub term_info: Option<TermInfo>,
}

pub struct PlAndInfo {
    pub pl: Rc<PostingsList>,
    pub weight: f32,
    pub include_in_proximity_ranking: bool,
    pub is_mandatory: bool,
    pub is_subtracted: bool,
    pub is_inverted: bool,
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
    pub fn iter(
        &self,
        weight: f32,
        include_in_proximity_ranking: bool,
        is_mandatory: bool,
        is_subtracted: bool,
        is_inverted: bool,
    ) -> PlIterator {
        PlIterator {
            prev_td: None,
            td: self.term_docs.get(0),
            pl: &self,
            idx: 0,
            weight,
            include_in_proximity_ranking,
            is_mandatory,
            is_subtracted,
            is_inverted,
        }
    }

    // Used for "processed" (e.g. phrase, bracket, AND) postings lists
    pub fn calc_pseudo_idf(&mut self, num_docs: u32) {
        self.idf = get_idf(num_docs as f32, self.term_docs.len() as f32);
    }

    pub fn parse_pl(
        &mut self,
        pl_vec: &[u8],
        invalidation_vector: &[u8],
        num_scored_fields: usize,
        with_positions: bool,
    ) {
        let term_info = unsafe { self.term_info.as_ref().unwrap_unchecked() };

        let mut pos = term_info.postings_file_offset as usize;

        self.term_docs.reserve_exact(term_info.doc_freq as usize);

        let mut prev_doc_id = 0;
        for _i in 0..term_info.doc_freq {
            let docfreq = decode_var_int(&pl_vec, &mut pos);

            let mut term_doc = Doc {
                doc_id: prev_doc_id + docfreq,
                fields: Vec::with_capacity(num_scored_fields),
                score: 0.0,
            };
            prev_doc_id = term_doc.doc_id;

            let mut is_last: u8 = 0;
            while is_last == 0 {
                debug_assert!(pos < pl_vec.len());

                let next_int = unsafe { *pl_vec.get_unchecked(pos) };
                pos += 1;

                is_last = next_int & LAST_FIELD_MASK;

                let (field_id, field_tf) = if (next_int & SHORT_FORM_MASK) != 0 {
                    ((next_int & 0b00111000) >> 3, (next_int & 0b00000111) as u32)
                } else {
                    (next_int & 0b00111111, decode_var_int(&pl_vec, &mut pos))
                };

                let field_positions = if with_positions {
                    /*
                     Positions are encoded with one of 2 schemes. See PostingsStreamReader.
                     */
                    let mut field_positions = Vec::with_capacity(field_tf as usize);

                    if field_tf >= MIN_CHUNK_SIZE {
                        let mut bit_pos = 0;

                        let num_chunks = (field_tf / CHUNK_SIZE)
                            + if field_tf % CHUNK_SIZE == 0 { 0 } else { 1 };

                        debug_assert!(pos <= pl_vec.len());

                        let slice_starting_here = unsafe { pl_vec.get_unchecked(pos..) };
                        let mut prev_pos = 0;
                        let mut read = 0;
                        for _chunk in 0..num_chunks {
                            // Read position length in this chunk
                            let chunk_len = read_bits_from(&mut bit_pos, 5, slice_starting_here) as usize;

                            for _i in 0..CHUNK_SIZE {
                                prev_pos += read_bits_from(&mut bit_pos, chunk_len, slice_starting_here);
                                field_positions.push(prev_pos);

                                read += 1;
                                if read == field_tf {
                                    break;
                                }
                            }
                        }

                        pos += (bit_pos / 8) + if bit_pos % 8 == 0 { 0 } else { 1 };
                    } else {
                        let mut prev_pos = 0;
                        for _j in 0..field_tf {
                            prev_pos += decode_var_int(&pl_vec, &mut pos);
                            field_positions.push(prev_pos);
                        }
                    }

                    field_positions
                } else {
                    Vec::new()
                };

                for _field_id_before in term_doc.fields.len() as u8..field_id {
                    term_doc.fields.push(Field::default());
                }

                term_doc.fields.push(Field { field_tf: field_tf as f32, field_positions });
            }

            if !infisearch_common::bitmap::check(invalidation_vector, prev_doc_id as usize) {
                self.term_docs.push(term_doc);
            }
        }
    }
}


impl PostingsList {
    pub fn merge_term_docs(term_doc_1: &Doc, term_doc_2: &Doc) -> Doc {
        let max_fields_length = std::cmp::max(term_doc_1.fields.len(), term_doc_2.fields.len());

        let mut td = Doc { doc_id: term_doc_1.doc_id, fields: Vec::with_capacity(max_fields_length), score: 0.0 };

        for field_id in 0..max_fields_length {
            let term_doc_1_field_opt = term_doc_1.fields.get(field_id);
            let term_doc_2_field_opt = term_doc_2.fields.get(field_id);

            if let Some(term_doc_1_field) = term_doc_1_field_opt {
                if let Some(term_doc_2_field) = term_doc_2_field_opt {
                    let mut doc_field = Field {
                        field_tf: term_doc_1_field.field_tf + term_doc_2_field.field_tf,
                        field_positions: Vec::with_capacity(
                            term_doc_1_field.field_positions.len() + term_doc_2_field.field_positions.len()
                        ),
                    };

                    let mut pos2_idx = 0;
                    for &pos1 in term_doc_1_field.field_positions.iter() {
                        // Guarantee: pos2_idx is at most term_doc_2_field.field_positions.len() - 1 at this point
                        while pos2_idx < term_doc_2_field.field_positions.len()
                            && unsafe { *term_doc_2_field.field_positions.get_unchecked(pos2_idx) } < pos1
                        {
                            doc_field.field_positions.push(
                                unsafe { *term_doc_2_field.field_positions.get_unchecked(pos2_idx) }
                            );
                            pos2_idx += 1;
                        }

                        // Guarantee: pos2_idx is at most term_doc_2_field.field_positions.len() - 1 at this point
                        if pos2_idx < term_doc_2_field.field_positions.len()
                            && unsafe { *term_doc_2_field.field_positions.get_unchecked(pos2_idx) } == pos1
                        {
                            pos2_idx += 1;
                        }

                        doc_field.field_positions.push(pos1);
                    }

                    // Guarantee: pos2_idx is at most term_doc_2_field.field_positions.len() at this point
                    for &p in unsafe { term_doc_2_field.field_positions.get_unchecked(pos2_idx..) } {
                        doc_field.field_positions.push(p);
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
}


#[cfg(test)]
pub mod test {
    use pretty_assertions::assert_eq;

    use super::{Field, PostingsList, Doc};

    // Takes a vector of "TermDoc", containing a vector of "fields", containing a tuple of (field_tf, vector of field positions)
    // E.g. a TermDoc containing 2 fields of term frequency 2 and 1: [ [2,[1,2]], [1,[120]] ]
    pub fn to_pl(term: Option<String>, text: &str) -> PostingsList {
        let vec: Vec<Option<Vec<(f32, Vec<u32>)>>> = miniserde::json::from_str(&format!("[{}]", text)).unwrap();

        let term_docs: Vec<Doc> = vec
            .into_iter()
            .enumerate()
            .filter(|doc_fields| doc_fields.1.is_some())
            .map(|doc_fields| (doc_fields.0, doc_fields.1.unwrap()))
            .map(|(doc_id, doc_fields)| vec_to_term_doc(doc_id as u32, doc_fields))
            .collect();

        PostingsList {
            term_docs,
            idf: 1.0,
            term,
            term_info: None,
        }
    }

    fn vec_to_term_doc(doc_id: u32, doc_fields: Vec<(f32, Vec<u32>)>) -> Doc {
        let fields: Vec<Field> = doc_fields
            .into_iter()
            .map(|(field_tf, field_positions)| Field { field_tf, field_positions })
            .collect();

        Doc { doc_id, fields, score: 0.0 }
    }

    pub fn to_pl_rc(text: &str) -> PostingsList {
        to_pl(None, text)
    }

    fn to_term_doc(text: &str) -> Doc {
        let doc_fields: Vec<(f32, Vec<u32>)> = miniserde::json::from_str(text).unwrap();
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
