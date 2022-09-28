use byteorder::{ByteOrder, LittleEndian};
#[cfg(feature = "indexer")]
use serde::{Serialize, Deserialize};

pub mod bitmap;
pub mod dictionary;
pub mod packed_var_int;
pub mod postings_list;
pub mod tokenize;
pub mod utils;

use dictionary::Dictionary;
use utils::varint;

pub static FILE_EXT: &str = "mls";
pub static METADATA_FILE: &str = "metadata.json";

pub struct MetadataReader {
    buf: Vec<u8>,
    dict_table_offset: usize,
    invalidation_vec_offset: usize,
    doc_infos_offset: usize,
    doc_infos_pos: usize,
}

impl MetadataReader {
    pub fn new(buf: Vec<u8>) -> Self {
        let dict_table_offset = LittleEndian::read_u32(&buf) as usize;
        let invalidation_vec_offset = LittleEndian::read_u32(&buf[4..]) as usize;
        let doc_infos_offset = LittleEndian::read_u32(&buf[8..]) as usize;

        MetadataReader {
            buf,
            dict_table_offset,
            invalidation_vec_offset,
            doc_infos_offset,
            doc_infos_pos: 0,
        }
    }
}

impl MetadataReader {
    pub fn get_invalidation_vec(&self, output: &mut Vec<u8>) {
        output.extend(&self.buf[self.invalidation_vec_offset..self.doc_infos_offset]);
    }

    pub fn read_docinfo_inital_metadata(
        &mut self,
        num_docs: &mut u32, doc_id_counter: &mut u32,
        average_lengths: &mut Vec<f64>,
        num_fields: usize,
    ) {
        self.doc_infos_pos = self.doc_infos_offset;

        *num_docs = LittleEndian::read_u32(&self.buf[self.doc_infos_pos..]);
        self.doc_infos_pos += 4;
        *doc_id_counter = LittleEndian::read_u32(&self.buf[self.doc_infos_pos..]);
        self.doc_infos_pos += 4;

        for _i in 0..num_fields {
            average_lengths.push(LittleEndian::read_f64(&self.buf[self.doc_infos_pos..]));
            self.doc_infos_pos += 8;
        }
    }

    #[inline(always)]
    pub fn read_docinfo_field_length(&mut self) -> u32 {
        varint::decode_var_int(&self.buf, &mut self.doc_infos_pos)
    }

    pub fn setup_dictionary(&self) -> Dictionary {
        dictionary::setup_dictionary(
            &self.buf[self.dict_table_offset..self.invalidation_vec_offset],
            &self.buf[12..self.dict_table_offset],
        )
    }
}

#[cfg(feature = "indexer")]
fn get_default_language() -> String {
    "ascii".to_owned()
}

#[cfg_attr(feature = "indexer", derive(Serialize, Deserialize))]
pub struct MorselsLanguageConfigOpts {
    pub stop_words: Option<Vec<String>>,
    pub ignore_stop_words: Option<bool>,
    pub stemmer: Option<String>,
    pub max_term_len: Option<usize>,
}

#[cfg(feature = "indexer")]
impl Default for MorselsLanguageConfigOpts {
    fn default() -> Self {
        MorselsLanguageConfigOpts {
            stop_words: None,
            ignore_stop_words: None,
            stemmer: None,
            max_term_len: None,
        }
    }
}

#[cfg_attr(feature = "indexer", derive(Serialize, Deserialize))]
pub struct MorselsLanguageConfig {
    #[cfg_attr(feature = "indexer", serde(default = "get_default_language"))]
    pub lang: String,

    #[cfg_attr(feature = "indexer", serde(default))]
    pub options: MorselsLanguageConfigOpts,
}

#[cfg(feature = "indexer")]
impl Default for MorselsLanguageConfig {
    fn default() -> Self {
        MorselsLanguageConfig {
            lang: get_default_language(),
            options: MorselsLanguageConfigOpts::default(),
        }
    }
}
