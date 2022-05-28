use std::collections::BTreeMap;
use std::iter::FromIterator;

use smartstring::alias::String;

use crate::tokenize::TermInfo;
use crate::utils::varint;

pub static DICTIONARY_STRING_FILE_NAME: &str = "dictionary_string.json";

pub struct Dictionary {
    pub term_infos: BTreeMap<String, TermInfo>,
}

struct DictionaryConstructor<'a> {
    table_vec: &'a [u8],
    string_vec: Vec<u8>,
    postings_file_name: u32,
    postings_file_offset: u32,
    dict_string_pos: usize,
    dict_table_pos: usize,
    prev_term: String,
}

/// An iterator to avoid double collecting into Vec during BTreeMap::from_iter
impl<'a> Iterator for DictionaryConstructor<'a> {
    type Item = (String, TermInfo);

    fn next(&mut self) -> Option<Self::Item> {
        if self.dict_table_pos >= self.table_vec.len() {
            return None;
        }

        let mut doc_freq = varint::decode_var_int(self.table_vec, &mut self.dict_table_pos);

        // new postings list delimiter
        if doc_freq == 0 {
            self.postings_file_name += 1;
            self.postings_file_offset = 0;
            doc_freq = varint::decode_var_int(self.table_vec, &mut self.dict_table_pos);
        }

        self.postings_file_offset += varint::decode_var_int(self.table_vec, &mut self.dict_table_pos);

        let prefix_len = self.string_vec[self.dict_string_pos] as usize;
        self.dict_string_pos += 1;

        let remaining_len = self.string_vec[self.dict_string_pos] as usize;
        self.dict_string_pos += 1;

        let term = String::from(&self.prev_term[..prefix_len])
            + unsafe {
                std::str::from_utf8_unchecked(
                    &self.string_vec[self.dict_string_pos..self.dict_string_pos + remaining_len],
                )
            };
        self.dict_string_pos += remaining_len;

        let ret = Some((
            term.clone(),
            TermInfo {
                doc_freq,
                postings_file_name: self.postings_file_name,
                postings_file_offset: self.postings_file_offset,
            },
        ));

        self.prev_term = term;

        ret
    }
}

pub fn setup_dictionary(
    table_vec: &[u8],
    string_vec: Vec<u8>,
) -> Dictionary {
    let term_infos = BTreeMap::from_iter(DictionaryConstructor {
        table_vec,
        string_vec,
        postings_file_name: 0,
        postings_file_offset: 0,
        dict_string_pos: 0,
        dict_table_pos: 0,
        prev_term: String::from(""),
    });

    Dictionary { term_infos }
}

impl Dictionary {
    pub fn get_term_info(&self, term: &str) -> Option<&TermInfo> {
        self.term_infos.get(&String::from(term))
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use pretty_assertions::assert_eq;
    use smartstring::alias::String;

    use crate::tokenize::TermInfo;

    #[test]
    fn test_dictionary_setup() {
        let dictionary = super::setup_dictionary(
                // Format: doc freq var-int, then pl offset var-int
            &[129, 127, 127, 131,
                129, 127, 127, 131,
                128,                // doc freq 0 is a new pl file delimiter
                129, 127, 127, 131,
                129, 127, 127, 131],
            {
                let mut string_vec = Vec::new();

                string_vec.extend(&[0, 3]);
                string_vec.extend("foo".as_bytes());

                string_vec.extend(&[3, 3]);
                string_vec.extend("bar".as_bytes());

                string_vec.extend(&[0, 4]);
                string_vec.extend("test".as_bytes());

                string_vec.extend(&[2, 4]);
                string_vec.extend("test".as_bytes());

                string_vec
            },
        );

        assert_eq!(dictionary.term_infos, {
            let mut terms = BTreeMap::default();

            terms.insert(
                String::from("foo"),
                TermInfo {
                    doc_freq: 1,
                    postings_file_name: 0,
                    postings_file_offset: 65535,
                },
            );

            terms.insert(
                String::from("foobar"),
                TermInfo {
                    doc_freq: 1,
                    postings_file_name: 0,
                    postings_file_offset: 65535 + 65535,
                },
            );

            terms.insert(
                String::from("test"),
                TermInfo {
                    doc_freq: 1,
                    postings_file_name: 1,
                    postings_file_offset: 65535,
                },
            );

            terms.insert(
                String::from("tetest"),
                TermInfo {
                    doc_freq: 1,
                    postings_file_name: 1,
                    postings_file_offset: 65535 + 65535,
                },
            );

            terms
        })
    }
}
