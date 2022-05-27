use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub mod bitmap;
pub mod dictionary;
pub mod tokenize;
pub mod utils;

pub static FILE_EXT: &str = "json";
pub static BITMAP_DOCINFO_DICT_TABLE_FILE: &str = "bitmap_docinfo_dicttable.json";

pub struct BitmapDocinfoDicttableReader {
    pub buf: Vec<u8>,
    pub pos: usize,
}

impl BitmapDocinfoDicttableReader {
    pub fn read_invalidation_vec(&mut self, output: &mut Vec<u8>) {
        let invalidation_vec_size = LittleEndian::read_u32(&self.buf) as usize;
        self.pos += 4;
        output.extend(&self.buf[self.pos..(self.pos + invalidation_vec_size)]);
        self.pos += invalidation_vec_size;
    }

    pub fn read_docinfo_inital_metadata(
        &mut self,
        num_docs: &mut u32, doc_id_counter: &mut u32,
        average_lengths: &mut Vec<f64>,
        num_fields: usize,
    ) {
        *num_docs = LittleEndian::read_u32(&self.buf[self.pos..]);
        self.pos += 4;
        *doc_id_counter = LittleEndian::read_u32(&self.buf[self.pos..]);
        self.pos += 4;

        for _i in 0..num_fields {
            average_lengths.push(LittleEndian::read_f64(&self.buf[self.pos..]));
            self.pos += 8;
        }
    }

    #[inline(always)]
    pub fn read_field_length(&mut self) -> u32 {
        let field_length = LittleEndian::read_u32(&self.buf[self.pos..]);
        self.pos += 4;
        field_length
    }

    pub fn get_dicttable_slice(&self) -> &[u8] {
        &self.buf[self.pos..]
    }
}

fn get_default_language() -> String {
    "ascii".to_owned()
}

#[derive(Serialize, Deserialize)]
pub struct MorselsLanguageConfig {
    #[serde(default = "get_default_language")]
    pub lang: String,

    #[serde(default)]
    pub options: Map<String, Value>,
}

impl Default for MorselsLanguageConfig {
    fn default() -> Self {
        MorselsLanguageConfig { lang: get_default_language(), options: Map::default() }
    }
}
