
use crate::utils::varint::PackedVarIntWriter;
use morsels_common::dictionary::{DICT_MAX_BIT_LENS, DICT_MAX_VALUES};

pub type DictTableWriter = PackedVarIntWriter::<4>; 

pub fn new() -> DictTableWriter {
    PackedVarIntWriter::<4>::new(DICT_MAX_BIT_LENS, DICT_MAX_VALUES)
}

impl DictTableWriter {
    pub fn write_doc_freq(&mut self, doc_freq: u32) {
        self.write_type(0, doc_freq);
    }

    pub fn write_pl_offset(&mut self, pl_offset: u32) {
        self.write_type(1, pl_offset);
    }

    pub fn write_prefix_len(&mut self, prefix_len: u8) {
        self.write_type(2, prefix_len as u32);
    }

    pub fn write_term_len(&mut self, term_len: u8) {
        self.write_type(3, term_len as u32);
    }
}
