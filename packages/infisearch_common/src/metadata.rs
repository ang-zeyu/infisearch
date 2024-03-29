use byteorder::{ByteOrder, LittleEndian};

use crate::dictionary::{self, Dictionary};
use crate::packed_var_int;
use crate::utils::{push, varint};

pub type EnumMax = u8;

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
        num_docs: &mut u32,
        doc_id_counter: &mut u32,
        average_lengths: &mut Vec<f64>,
        num_enum_fields: &mut usize,
        num_i64_fields: &mut usize,
        num_fields: usize,
    ) -> (Vec<EnumMax>, Vec<i64>) {
        self.doc_infos_pos = self.doc_infos_offset;

        *num_docs = LittleEndian::read_u32(&self.buf[self.doc_infos_pos..]);
        self.doc_infos_pos += 4;
        *doc_id_counter = LittleEndian::read_u32(&self.buf[self.doc_infos_pos..]);
        self.doc_infos_pos += 4;

        for _i in 0..num_fields {
            push::push_wo_grow(
                average_lengths,
                LittleEndian::read_f64(&self.buf[self.doc_infos_pos..]),
            );
            self.doc_infos_pos += 8;
        }

        let mut doc_infos_enum_pos = self.doc_infos_pos
            + LittleEndian::read_u32(&self.buf[self.doc_infos_pos..]) as usize;
        self.doc_infos_pos += 4;

        debug_assert!(doc_infos_enum_pos <= self.buf.len());
        *num_enum_fields = LittleEndian::read_u32(unsafe { self.buf.get_unchecked(doc_infos_enum_pos..) }) as usize;
        doc_infos_enum_pos += 4;

        debug_assert!(doc_infos_enum_pos <= self.buf.len());
        let doc_infos_enum_bit_lens = unsafe { self.buf.get_unchecked(doc_infos_enum_pos..) };
        doc_infos_enum_pos += *num_enum_fields;

        debug_assert!(doc_infos_enum_pos <= self.buf.len());
        let doc_infos_enum_ev_ids = unsafe { self.buf.get_unchecked(doc_infos_enum_pos..) };

        let mut doc_enum_vals: Vec<EnumMax> = Vec::with_capacity(*num_enum_fields * *doc_id_counter as usize);
        let mut doc_infos_enum_bit_r_pos = 0;
        for _doc_id in 0..*doc_id_counter {
            for enum_id in 0..*num_enum_fields {
                debug_assert!(enum_id < doc_infos_enum_bit_lens.len());
        
                let bits_used = unsafe { *doc_infos_enum_bit_lens.get_unchecked(enum_id) } as usize;
                let ev_id = packed_var_int::read_bits_from(
                    &mut doc_infos_enum_bit_r_pos, bits_used,
                    doc_infos_enum_ev_ids,
                ) as EnumMax;
                push::push_wo_grow(&mut doc_enum_vals, ev_id);
            }
        }

        let mut doc_infos_num_pos = doc_infos_enum_pos
            + doc_infos_enum_bit_r_pos / 8
            + if doc_infos_enum_bit_r_pos % 8 == 0 { 0 } else { 1 };

        debug_assert!(doc_infos_num_pos <= self.buf.len());
        *num_i64_fields = LittleEndian::read_u32(unsafe { self.buf.get_unchecked(doc_infos_num_pos..) }) as usize;
        doc_infos_num_pos += 4;

        let mut i64_min_vals: Vec<i64> = Vec::with_capacity(*num_i64_fields as usize);
        for _i64_id in 0..*num_i64_fields {
            debug_assert!(doc_infos_num_pos <= self.buf.len());
            push::push_wo_grow(
                &mut i64_min_vals,
                LittleEndian::read_i64(unsafe { &self.buf.get_unchecked(doc_infos_num_pos..) }),
            );
            doc_infos_num_pos += 8;
        }

        let mut doc_i64_vals: Vec<i64> = Vec::with_capacity(*num_i64_fields * *doc_id_counter as usize);
        for _doc_id in 0..*doc_id_counter {
            for i64_id in 0..*num_i64_fields {
                debug_assert!(doc_infos_num_pos <= self.buf.len());
                let delta = varint::decode_var_int_u64(&self.buf, &mut doc_infos_num_pos);
                let base = unsafe { *i64_min_vals.get_unchecked(i64_id) };
                let value = base.wrapping_add(delta as i64);

                push::push_wo_grow(&mut doc_i64_vals,value);
            }
        }

        (doc_enum_vals, doc_i64_vals)
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
