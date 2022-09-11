use std::io::Write;

use crate::utils::varint::PackedVarIntWriter;

use bitvec::{vec::BitVec, prelude::Msb0};
use morsels_common::dictionary::{DICT_MAX_BIT_LENS, DICT_MAX_VALUES};

type DictTableWriter = PackedVarIntWriter::<4>; 
type DictStringWriter = Vec<u8>; 

pub struct DictWriter {
    table_writer: DictTableWriter,
    string_writer: DictStringWriter,
}

impl DictWriter {
    pub fn new() -> Self {
        DictWriter {
            table_writer: PackedVarIntWriter::<4>::new(DICT_MAX_BIT_LENS, DICT_MAX_VALUES),
            string_writer: Vec::with_capacity(2048),
        }
    }

    fn write_doc_freq(&mut self, doc_freq: u32) {
        self.table_writer.write_type(0, doc_freq);
    }

    fn write_pl_offset(&mut self, pl_offset: u32) {
        self.table_writer.write_type(1, pl_offset);
    }

    fn write_prefix_len(&mut self, prefix_len: u8) {
        self.table_writer.write_type(2, prefix_len as u32);
    }

    fn write_term_len(&mut self, term_len: u8) {
        self.table_writer.write_type(3, term_len as u32);
    }

    pub fn write_dict_table_entry(
        &mut self,
        doc_freq: u32,
        start_pl_offset: u32, prev_pl_start_offset: &mut u32,
        prefix_len: u8, remaining_len: u8,
    ) {
        self.write_doc_freq(doc_freq);
        self.write_pl_offset(start_pl_offset - *prev_pl_start_offset);
        self.write_prefix_len(prefix_len);
        self.write_term_len(remaining_len);
        *prev_pl_start_offset = start_pl_offset;
    }

    pub fn write_pl_separator(&mut self) {
        self.write_doc_freq(0);
    }

    #[inline(always)]
    pub fn write_term(&mut self, prev_term: &str, curr_term: &str) -> (u8, u8) {
        let unicode_prefix_byte_len = get_common_unicode_prefix_byte_len(prev_term, curr_term);
    
        self.string_writer.write_all(&curr_term.as_bytes()[unicode_prefix_byte_len..]).unwrap();
    
        (
            unicode_prefix_byte_len as u8,                     // Prefix length
            (curr_term.len() - unicode_prefix_byte_len) as u8, // Remaining length
        )
    }

    pub fn flush(mut self) -> (BitVec<u8, Msb0>, Vec<u8>) {
        self.string_writer.flush().unwrap();
        (
            self.table_writer.flush(),
            self.string_writer,
        )
    }
}

#[inline(always)]
fn get_common_unicode_prefix_byte_len(str1: &str, str2: &str) -> usize {
    let mut byte_len = 0;
    let mut str1_it = str1.chars();
    let mut str2_it = str2.chars();

    loop {
        let str1_next = str1_it.next();
        let str2_next = str2_it.next();
        if str1_next == None || str2_next == None || (str1_next.unwrap() != str2_next.unwrap()) {
            break;
        }

        byte_len += str1_next.unwrap().len_utf8();
    }

    byte_len
}
