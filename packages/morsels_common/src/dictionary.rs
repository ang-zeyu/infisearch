use std::collections::BTreeMap;
use std::iter::FromIterator;

use smartstring::alias::String;

use crate::packed_var_int::PackedVarIntReader;

pub const DICT_MAX_BIT_LENS: [usize; 4] = [5, 5, 3, 3];
pub const DICT_MAX_VALUES: [usize; 4] = [4, 4, 8, 8];
pub const DICT_MAX_VALUES_U8: [u8; 4] = [4, 4, 8, 8];

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct TermInfo {
    pub doc_freq: u32,
    pub postings_file_name: u32,
    pub postings_file_offset: u32,
}

pub struct Dictionary {
    pub term_infos: BTreeMap<String, &'static TermInfo>,
}

struct DictionaryConstructor<'a> {
    table_rdr: PackedVarIntReader<'a, 4>,
    string_vec: &'a [u8],
    postings_file_name: u32,
    postings_file_offset: u32,
    dict_string_pos: usize,
    prev_term: String,
}

/// An iterator to avoid double collecting into Vec during BTreeMap::from_iter
impl<'a> Iterator for DictionaryConstructor<'a> {
    type Item = (String, &'static TermInfo);

    fn next(&mut self) -> Option<Self::Item> {
        if self.dict_string_pos >= self.string_vec.len() {
            return None;
        }

        let mut doc_freq = self.table_rdr.read_type(0);

        // new postings list delimiter
        if doc_freq == 0 {
            self.postings_file_name += 1;
            self.postings_file_offset = 0;
            doc_freq = self.table_rdr.read_type(0);
        }

        self.postings_file_offset += self.table_rdr.read_type(1);

        let prefix_len = self.table_rdr.read_type(2) as usize;
        let remaining_len = self.table_rdr.read_type(3) as usize;

        let term = String::from(&self.prev_term[..prefix_len])
            + unsafe {
                std::str::from_utf8_unchecked(
                    &self.string_vec[self.dict_string_pos..self.dict_string_pos + remaining_len],
                )
            };
        self.dict_string_pos += remaining_len;

        let term_info: &'static TermInfo = Box::leak(Box::new(TermInfo {
            doc_freq,
            postings_file_name: self.postings_file_name,
            postings_file_offset: self.postings_file_offset,
        }));

        let ret = Some((
            term.clone(),
            term_info
        ));

        self.prev_term = term;

        ret
    }
}

pub fn setup_dictionary(
    table_vec: &[u8],
    string_vec: &[u8],
) -> Dictionary {
    let table_rdr = PackedVarIntReader::<4>::new(
        table_vec,
        DICT_MAX_BIT_LENS,
        DICT_MAX_VALUES_U8,
    );

    let term_infos = BTreeMap::from_iter(DictionaryConstructor {
        table_rdr,
        string_vec,
        postings_file_name: 0,
        postings_file_offset: 0,
        dict_string_pos: 0,
        prev_term: String::from(""),
    });

    Dictionary { term_infos }
}

impl Dictionary {
    pub fn get_term_info(&self, term: &str) -> Option<&TermInfo> {
        self.term_infos.get(term).map(|ti| *ti)
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use pretty_assertions::assert_eq;
    use smartstring::alias::String;

    use super::TermInfo;

    #[test]
    fn test_dictionary_setup() {
        let mut string_vec = Vec::new();

        let dictionary = super::setup_dictionary(
            /*
             Format: doc freq, then pl offset, then prefix len, term len,
             all as packed variable integers (see dict_table_writer)

             00000_1_01__111_11111__11111111__111_001_00___010_011
             1_1__11111111__1111111_1__1_011
             0                                                // doc freq 0 is a new pl file delimiter
             1_11__11111111__111111_00___100
             00000___1_1111111__11111111__1_10_100
             */
            &[
                5, 255, 255, 228, 79, 255, 255, 183, 255, 252, 128, 255, 255, 208
            ],
            {

                string_vec.extend("foo".as_bytes());
                string_vec.extend("bar".as_bytes());
                string_vec.extend("test".as_bytes());
                string_vec.extend("test".as_bytes());

                &string_vec
            },
        );

        assert_eq!(dictionary.term_infos, {
            let mut terms = BTreeMap::default();

            let term_info: &'static TermInfo = Box::leak(Box::new(TermInfo {
                doc_freq: 1,
                postings_file_name: 0,
                postings_file_offset: 65535,
            }));
            terms.insert(
                String::from("foo"),
                term_info,
            );

            let term_info: &'static TermInfo = Box::leak(Box::new(TermInfo {
                doc_freq: 1,
                postings_file_name: 0,
                postings_file_offset: 65535 + 65535,
            }));
            terms.insert(
                String::from("foobar"),
                term_info,
            );

            let term_info: &'static TermInfo = Box::leak(Box::new(TermInfo {
                doc_freq: 1,
                postings_file_name: 1,
                postings_file_offset: 65535,
            }));
            terms.insert(
                String::from("test"),
                term_info,
            );

            let term_info: &'static TermInfo = Box::leak(Box::new(TermInfo {
                doc_freq: 1,
                postings_file_name: 1,
                postings_file_offset: 65535 + 65535,
            }));
            terms.insert(
                String::from("tetest"),
                term_info,
            );

            terms
        })
    }
}
