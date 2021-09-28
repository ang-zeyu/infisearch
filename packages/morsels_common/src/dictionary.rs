pub mod trigrams;

use std::rc::Rc;

use rustc_hash::FxHashMap;
use smartstring::alias::String;
use smartstring::alias::String as SmartString;

use crate::tokenize::TermInfo;
use crate::utils::varint;
use trigrams::get_tri_grams;

pub static DICTIONARY_TABLE_FILE_NAME: &str = "_dictionary_table";
pub static DICTIONARY_STRING_FILE_NAME: &str = "_dictionary_string";

pub struct Dictionary {
    pub term_infos: FxHashMap<Rc<String>, Rc<TermInfo>>,
    pub trigrams: FxHashMap<SmartString, Vec<Rc<String>>>,
}

#[inline(always)]
pub fn setup_dictionary(table_vec: Vec<u8>, string_vec: Vec<u8>, num_docs: u32, build_trigram: bool) -> Dictionary {
    let mut term_infos: FxHashMap<Rc<String>, Rc<TermInfo>> = FxHashMap::default();

    let mut postings_file_name = 0;
    let mut dict_string_pos = 0;
    let mut dict_table_pos = 0;
    let mut prev_term: Rc<String> = Rc::new(SmartString::from(""));

    let table_vec_len = table_vec.len();
    while dict_table_pos < table_vec_len {
        let doc_freq = varint::decode_var_int(&table_vec, &mut dict_table_pos);

        // new postings list delimiter
        if doc_freq == 0 {
            postings_file_name += 1;
            continue;
        }

        let postings_file_offset = varint::decode_var_int(&table_vec, &mut dict_table_pos);

        let prefix_len = string_vec[dict_string_pos] as usize;
        dict_string_pos += 1;

        let remaining_len = string_vec[dict_string_pos] as usize;
        dict_string_pos += 1;

        let term = Rc::new(
            SmartString::from(&prev_term[..prefix_len])
                + unsafe {
                    std::str::from_utf8_unchecked(&string_vec[dict_string_pos..dict_string_pos + remaining_len])
                },
        );
        dict_string_pos += remaining_len;

        term_infos.insert(
            Rc::clone(&term),
            Rc::new(TermInfo {
                doc_freq,
                idf: (1.0 + (num_docs as f64 - doc_freq as f64 + 0.5) / (doc_freq as f64 + 0.5)).ln(),
                postings_file_name,
                postings_file_offset,
            }),
        );

        prev_term = term;
    }

    let trigrams = if build_trigram { setup_trigrams(&term_infos) } else { FxHashMap::default() };

    Dictionary { term_infos, trigrams }
}

fn setup_trigrams(term_infos: &FxHashMap<Rc<String>, Rc<TermInfo>>) -> FxHashMap<SmartString, Vec<Rc<String>>> {
    let mut trigrams: FxHashMap<SmartString, Vec<Rc<String>>> = FxHashMap::default();

    for term in term_infos.keys() {
        for term_trigram in get_tri_grams(term) {
            match trigrams.get_mut(term_trigram) {
                Some(terms) => {
                    terms.push(Rc::clone(term));
                }
                None => {
                    let mut term_vec: Vec<Rc<String>> = Vec::with_capacity(20);
                    term_vec.push(Rc::clone(term));
                    trigrams.insert(SmartString::from(term_trigram), term_vec);
                }
            }
        }
    }

    trigrams
}

impl Dictionary {
    pub fn get_term_info(&self, term: &str) -> Option<&Rc<TermInfo>> {
        self.term_infos.get(&String::from(term))
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use pretty_assertions::assert_eq;
    use rustc_hash::FxHashMap;
    use smartstring::alias::String;

    use crate::tokenize::TermInfo;

    #[test]
    fn test_dictionary_setup() {
        let dictionary = super::setup_dictionary(
            vec![
                129, 255, 255, 0, 0,
                129, 255, 255, 0, 0,
                128,
                129, 255, 255, 0, 0, 
                129, 255, 255, 0, 0
            ],
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
            10,
            false
        );

        assert_eq!(
            dictionary.term_infos,
            {
                let mut terms = FxHashMap::default();

                terms.insert(Rc::new(String::from("foo")), Rc::new(TermInfo {
                    doc_freq: 1,
                    idf: 0.0,
                    postings_file_name: 0,
                    postings_file_offset: 65535,
                }));

                terms.insert(Rc::new(String::from("foobar")), Rc::new(TermInfo {
                    doc_freq: 1,
                    idf: 0.0,
                    postings_file_name: 0,
                    postings_file_offset: 65535,
                }));

                terms.insert(Rc::new(String::from("test")), Rc::new(TermInfo {
                    doc_freq: 1,
                    idf: 0.0,
                    postings_file_name: 1,
                    postings_file_offset: 65535,
                }));

                terms.insert(Rc::new(String::from("tetest")), Rc::new(TermInfo {
                    doc_freq: 1,
                    idf: 0.0,
                    postings_file_name: 1,
                    postings_file_offset: 65535,
                }));

                terms
            }
        )
    }
}
