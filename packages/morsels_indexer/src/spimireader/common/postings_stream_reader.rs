use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::str;
use std::sync::Arc;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam::channel::Sender;
use dashmap::DashMap;

use morsels_common::postings_list::{LAST_FIELD_MASK, SHORT_FORM_MASK};

use super::{PostingsStreamDecoder, TermDocsForMerge};
use crate::docinfo::DocInfos;
use crate::utils::varint;
use crate::worker::MainToWorkerMessage;

pub struct PostingsStreamReader {
    pub idx: u32,
    pub buffered_reader: BufReader<File>,
    pub buffered_dict_reader: BufReader<File>,
    pub future_term_buffer: VecDeque<TermDocsForMerge>,
    pub doc_infos_unlocked: Arc<DocInfos>,
    pub num_scored_fields: usize,
}

impl PostingsStreamReader {
    pub fn read_next_batch(
        self,
        n: usize,
        tx_main: &Sender<MainToWorkerMessage>,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
    ) {
        tx_main
            .send(MainToWorkerMessage::Decode {
                n,
                postings_stream_reader: self,
                postings_stream_decoders,
            })
            .expect("Failed to request worker spimi block decode!");
    }

    #[inline]
    fn read_and_write_doc(
        pl_reader: &mut BufReader<File>,
        combined_var_ints: &mut Vec<u8>,
        u8_buf: &mut [u8; 1],
        u32_buf: &mut [u8; 4],
        with_positions: bool,
        num_scored_fields: usize,
     ) {
        pl_reader.read_exact(u8_buf).unwrap();
        let num_fields = u8_buf[0];

        for _j in 1..num_fields {
            Self::read_and_write_field(
                pl_reader,
                combined_var_ints,
                u8_buf, u32_buf,
                false,
                with_positions,
                num_scored_fields,
            );
        }

        Self::read_and_write_field(
            pl_reader,
            combined_var_ints,
            u8_buf,
            u32_buf,
            true,
            with_positions,
            num_scored_fields
        );
    }

    #[inline]
    fn read_and_write_field(
        pl_reader: &mut BufReader<File>,
        combined_var_ints: &mut Vec<u8>,
        u8_buf: &mut [u8; 1],
        u32_buf: &mut [u8; 4],
        is_last: bool,
        with_positions: bool,
        num_scored_fields: usize,
    ) {
        pl_reader.read_exact(u8_buf).unwrap();

        // If it is the last field, mask with |= LAST_FIELD_MASK (instead of storing number of fields).
        let field_id = u8_buf[0];

        pl_reader.read_exact(u32_buf).unwrap();
        let field_tf = LittleEndian::read_u32(u32_buf);

        if num_scored_fields <= 8 && field_tf <= 7  {
            /*
            If the number of scored fields is <= 8,
            and the field term frequency is <= 7,
            also compress the field tf into this single byte like so:

            SHORT_FORM_MASK | field_id << 3 | field_tf
            | LAST_FIELD_MASK (if applicable)
             */
            let compressed_field_info = SHORT_FORM_MASK
                | (field_id << 3)
                | (field_tf as u8)
                | if is_last { LAST_FIELD_MASK } else { 0_u8 };

            combined_var_ints.push(compressed_field_info);
        } else {
            if is_last {
                combined_var_ints.push(field_id | LAST_FIELD_MASK);
            } else {
                combined_var_ints.push(field_id);
            }

            // Store field tf separately otherwise
            varint::get_var_int_vec(field_tf, combined_var_ints);
        }

        /*
            Pre-encode position gaps into varint in the worker,
            then write it out in the main thread later.
        */

        if with_positions {
            let mut prev_pos = 0;
            for _k in 0..field_tf {
                pl_reader.read_exact(u32_buf).unwrap();
                let curr_pos = LittleEndian::read_u32(u32_buf);
                varint::get_var_int_vec(curr_pos - prev_pos, combined_var_ints);
                prev_pos = curr_pos;
            }
        }
    }

    #[inline]
    pub fn decode_next_n(
        mut self,
        n: usize,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
        with_positions: bool,
    ) {
        let mut u32_buf: [u8; 4] = [0; 4];
        let mut u8_buf: [u8; 1] = [0; 1];

        let pl_reader = &mut self.buffered_reader;

        for _unused in 0..n {
            if let Ok(()) = self.buffered_dict_reader.read_exact(&mut u8_buf) {
                // Temporary combined dictionary table / dictionary string
                let mut term_vec = vec![0; u8_buf[0] as usize];
                self.buffered_dict_reader.read_exact(&mut term_vec).unwrap();
                let term = str::from_utf8(&term_vec)
                    .expect("Unexpected error, unable to parse utf8 string from temporary dictionary")
                    .to_owned();

                self.buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                let doc_freq = LittleEndian::read_u32(&u32_buf);

                // TODO improve the capacity heuristic
                let mut combined_var_ints = Vec::with_capacity((doc_freq * 20) as usize);

                /*
                For the first document, don't encode the doc id variable integer.
                Encode it in the main thread later where the gap information between blocks is available.
                */
                pl_reader.read_exact(&mut u32_buf).unwrap();
                let first_doc_id = LittleEndian::read_u32(&u32_buf);

                let mut prev_doc_id = first_doc_id;
                Self::read_and_write_doc(
                    pl_reader,
                    &mut combined_var_ints,
                    &mut u8_buf,
                    &mut u32_buf,
                    with_positions,
                    self.num_scored_fields,
                );

                for _i in 1..doc_freq {
                    pl_reader.read_exact(&mut u32_buf).unwrap();
                    let doc_id = LittleEndian::read_u32(&u32_buf);
                    varint::get_var_int_vec(doc_id - prev_doc_id, &mut combined_var_ints);

                    prev_doc_id = doc_id;
                    Self::read_and_write_doc(
                        pl_reader,
                        &mut combined_var_ints,
                        &mut u8_buf,
                        &mut u32_buf,
                        with_positions, 
                        self.num_scored_fields,
                    );
                }

                self.future_term_buffer.push_back(TermDocsForMerge {
                    term,
                    doc_freq,
                    combined_var_ints,
                    first_doc_id,
                    last_doc_id: prev_doc_id,
                });
            } else {
                break; // eof
            }
        }

        {
            let mut postings_stream_decoder_entry = postings_stream_decoders.get_mut(&self.idx).unwrap();
            let postings_stream_decoder = postings_stream_decoder_entry.value_mut();
            match postings_stream_decoder {
                PostingsStreamDecoder::None => {
                    *postings_stream_decoder = PostingsStreamDecoder::Reader(self);
                }
                PostingsStreamDecoder::Notifier(_tx) => {
                    let notifier_decoder = std::mem::replace(
                        postings_stream_decoder,
                        PostingsStreamDecoder::Reader(self),
                    );

                    // Main thread was blocked as this worker was still decoding
                    // Re-notify that decoding is done!
                    if let PostingsStreamDecoder::Notifier(tx) = notifier_decoder {
                        tx.send(()).unwrap();
                    }
                }
                PostingsStreamDecoder::Reader(_r) => panic!("Reader still available in array @worker"),
            }
        }
    }
}
