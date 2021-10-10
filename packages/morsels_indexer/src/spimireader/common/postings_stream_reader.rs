use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::str;
use std::sync::Arc;

use byteorder::{ByteOrder, LittleEndian};
use dashmap::DashMap;

use super::{PostingsStreamDecoder, TermDocsForMerge};
use crate::docinfo::DocInfos;
use crate::utils::varint;
use crate::FieldInfos;
use crate::MainToWorkerMessage;
use crate::Sender;

static POSTINGS_STREAM_BUFFER_SIZE: u32 = 5000;

pub struct PostingsStreamReader {
    pub idx: u32,
    pub buffered_reader: BufReader<File>,
    pub buffered_dict_reader: BufReader<File>,
    pub future_term_buffer: VecDeque<TermDocsForMerge>,
    pub doc_infos_unlocked: Arc<DocInfos>,
}

static LAST_FIELD_MASK: u8 = 0x80; // 1000 0000

impl PostingsStreamReader {
    pub fn read_next_batch(
        self,
        tx_main: &Sender<MainToWorkerMessage>,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
    ) {
        tx_main
            .send(MainToWorkerMessage::Decode {
                n: POSTINGS_STREAM_BUFFER_SIZE,
                postings_stream_reader: self,
                postings_stream_decoders,
            })
            .expect("Failed to request worker spimi block decode!");
    }

    #[inline]
    pub fn decode_next_n(
        mut self,
        n: u32,
        postings_stream_decoders: Arc<DashMap<u32, PostingsStreamDecoder>>,
        with_positions: bool,
        field_infos: &Arc<FieldInfos>,
    ) {
        let mut u32_buf: [u8; 4] = [0; 4];
        let mut u8_buf: [u8; 1] = [0; 1];

        let pl_reader = &mut self.buffered_reader;
        let doc_infos = &self.doc_infos_unlocked;

        for _unused in 0..n {
            if let Ok(()) = self.buffered_dict_reader.read_exact(&mut u8_buf) {
                // Temporary combined dictionary table / dictionary string
                let mut term_vec = vec![0; u8_buf[0] as usize];
                self.buffered_dict_reader.read_exact(&mut term_vec).unwrap();
                let term = str::from_utf8(&term_vec).unwrap().to_owned();

                self.buffered_dict_reader.read_exact(&mut u32_buf).unwrap();
                let doc_freq = LittleEndian::read_u32(&u32_buf);

                // TODO improve the capacity heuristic
                let mut combined_var_ints = Vec::with_capacity((doc_freq * 20) as usize);

                let mut max_doc_term_score: f32 = 0.0;

                let mut read_and_write_doc =
                    |doc_id,
                     pl_reader: &mut BufReader<File>,
                     combined_var_ints: &mut Vec<u8>,
                     u8_buf: &mut [u8; 1],
                     u32_buf: &mut [u8; 4]| {
                        let mut curr_doc_term_score: f32 = 0.0;
                        let mut read_and_write_field =
                            |field_id,
                             pl_reader: &mut BufReader<File>,
                             combined_var_ints: &mut Vec<u8>,
                             u32_buf: &mut [u8; 4]| {
                                pl_reader.read_exact(u32_buf).unwrap();
                                let field_tf = LittleEndian::read_u32(u32_buf);
                                varint::get_var_int_vec(field_tf, combined_var_ints);

                                /*
                                    Pre-encode field tf and position gaps into varint in the worker,
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

                                let field_info = field_infos.field_infos_by_id.get(field_id as usize).unwrap();
                                let k = field_info.k;
                                let b = field_info.b;
                                curr_doc_term_score += (field_tf as f32 * (k + 1.0))
                                    / (field_tf as f32
                                        + k * (1.0 - b
                                            + b * (doc_infos
                                                .get_field_len_factor(doc_id as usize, field_id as usize))))
                                    * field_info.weight;
                            };

                        pl_reader.read_exact(u8_buf).unwrap();
                        let num_fields = u8_buf[0];
                        for _j in 1..num_fields {
                            pl_reader.read_exact(u8_buf).unwrap();
                            let field_id = u8_buf[0];
                            combined_var_ints.push(field_id);

                            read_and_write_field(field_id, pl_reader, combined_var_ints, u32_buf);
                        }

                        // Delimit the last field with LAST_FIELD_MASK
                        pl_reader.read_exact(u8_buf).unwrap();
                        let field_id = u8_buf[0];
                        combined_var_ints.push(field_id | LAST_FIELD_MASK);
                        read_and_write_field(field_id, pl_reader, combined_var_ints, u32_buf);

                        if curr_doc_term_score > max_doc_term_score {
                            max_doc_term_score = curr_doc_term_score;
                        }
                    };

                /*
                For the first document, don't encode the doc id variable integer.
                Encode it in the main thread later where the gap information between blocks is available.
                */
                pl_reader.read_exact(&mut u32_buf).unwrap();
                let first_doc_id = LittleEndian::read_u32(&u32_buf);

                let mut prev_doc_id = first_doc_id;
                read_and_write_doc(
                    first_doc_id,
                    pl_reader,
                    &mut combined_var_ints,
                    &mut u8_buf,
                    &mut u32_buf,
                );

                for _i in 1..doc_freq {
                    pl_reader.read_exact(&mut u32_buf).unwrap();
                    let doc_id = LittleEndian::read_u32(&u32_buf);
                    varint::get_var_int_vec(doc_id - prev_doc_id, &mut combined_var_ints);

                    prev_doc_id = doc_id;
                    read_and_write_doc(doc_id, pl_reader, &mut combined_var_ints, &mut u8_buf, &mut u32_buf);
                }

                self.future_term_buffer.push_back(TermDocsForMerge {
                    term,
                    max_doc_term_score,
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
                        tx.lock().unwrap().send(()).unwrap();
                    }
                }
                PostingsStreamDecoder::Reader(_r) => panic!("Reader still available in array @worker"),
            }
        }
    }
}
