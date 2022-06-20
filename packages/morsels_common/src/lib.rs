use byteorder::{ByteOrder, LittleEndian};
use miniserde::json as mini_json;
#[cfg(not(feature = "indexer"))]
use miniserde::Deserialize as MiniDeserialize;
#[cfg(feature = "indexer")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

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

#[cfg(feature = "indexer")]
fn serdejson_to_miniserde<'de, D>(deserializer: D) -> Result<mini_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    let v: serde_json::Value = Deserialize::deserialize(deserializer)?;

    // Reserialize with serde_json then deserialize with miniserde
    let serialized = serde_json::to_string(&v)
        .expect("Temporary serde_json serialization should not fail");

    if let Ok(config) = mini_json::from_str(&serialized) {
        Ok(config)
    } else {
        Err(D::Error::custom("Language Configuration is invalid"))
    }
}

#[cfg(feature = "indexer")]
fn miniserde_to_serdejson<S: Serializer>(v :&mini_json::Value, serializer: S) -> Result<S::Ok, S::Error>
{
    let v: serde_json::Value = serde_json::from_str(&mini_json::to_string(v)).unwrap();
    v.serialize(serializer)
}

#[cfg_attr(not(feature = "indexer"), derive(MiniDeserialize))]
#[cfg_attr(feature = "indexer", derive(Serialize, Deserialize))]
pub struct MorselsLanguageConfig {
    pub lang: String,

    #[cfg_attr(
        feature = "indexer",
        serde(serialize_with = "miniserde_to_serdejson", deserialize_with = "serdejson_to_miniserde"),
    )]
    pub options: mini_json::Value,
}

impl Default for MorselsLanguageConfig {
    fn default() -> Self {
        MorselsLanguageConfig {
            lang: get_default_language(),
            options: mini_json::Value::Object(mini_json::Object::default()),
        }
    }
}
