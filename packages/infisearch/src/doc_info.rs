use std::cmp::Ordering;
use std::iter::FromIterator;
use std::sync::Arc;

use crate::field_info::EnumInfo;
use crate::field_info::FieldInfos;
use crate::i_debug;
use crate::incremental_info::IncrementalIndexInfo;
use crate::utils::varint;
use crate::worker::miner::WorkerMinerDocInfo;
use std::io::Write;

use bitvec::prelude::Msb0;
use bitvec::vec::BitVec;
use bitvec::view::BitView;
use log::warn;
use infisearch_common::EnumMax;
use infisearch_common::MetadataReader;
use infisearch_common::bitmap;
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct BlockDocLengths(pub Vec<WorkerMinerDocInfo>);

impl Eq for BlockDocLengths {}

impl PartialEq for BlockDocLengths {
    fn eq(&self, other: &Self) -> bool {
        self.0[0].doc_id == other.0[0].doc_id
    }
}

impl Ord for BlockDocLengths {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0[0].doc_id.cmp(&other.0[0].doc_id)
    }
}

impl PartialOrd for BlockDocLengths {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0[0].doc_id.cmp(&other.0[0].doc_id))
    }
}

#[derive(Debug)]
pub struct DocInfos {
    pub doc_infos: Vec<WorkerMinerDocInfo>,
    pub all_block_doc_lengths: Vec<BlockDocLengths>, // store doc lengths from each block and sort later
    average_lengths: Vec<f64>,
    docs_enum_values: Vec<EnumMax>,
    docs_i64_values: Vec<i64>,
    original_doc_id_counter: u32,
}

impl DocInfos {
    pub fn init_doc_infos(
        field_infos: &Arc<FieldInfos>,
        metadata_rdr: Option<&mut MetadataReader>,
    ) -> DocInfos {
        let num_scored_fields = field_infos.num_scored_fields;
        if metadata_rdr.is_none() {
            // Full index
            return DocInfos {
                doc_infos: Vec::new(),
                all_block_doc_lengths: Vec::new(),
                average_lengths: vec![0.0; num_scored_fields],
                docs_enum_values: Vec::new(),
                docs_i64_values: Vec::new(),
                original_doc_id_counter: 0,
            };
        }

        let metadata_rdr = unsafe { metadata_rdr.unwrap_unchecked() };
        let mut doc_id_counter = 0;
        // Capacity must be set
        let mut average_lengths: Vec<f64> = Vec::with_capacity(num_scored_fields);
        let (docs_enum_values, docs_i64_values) = metadata_rdr.read_docinfo_inital_metadata(
            &mut 0, &mut doc_id_counter, &mut average_lengths,
            &mut 0, &mut 0, num_scored_fields,
        );

        let mut doc_lengths = Vec::with_capacity(doc_id_counter as usize);

        for doc_id in 0..doc_id_counter {
            let mut doc_info = WorkerMinerDocInfo {
                doc_id,
                doc_enums: Vec::new(),
                doc_nums: Vec::new(),
                field_lengths: Vec::with_capacity(num_scored_fields),
                field_texts: Vec::new(),
            };

            for _i in 0..num_scored_fields {
                doc_info.field_lengths.push(metadata_rdr.read_docinfo_field_length());
            }

            doc_lengths.push(doc_info);
        }

        DocInfos {
            doc_infos: doc_lengths,
            all_block_doc_lengths: Vec::new(),
            average_lengths,
            docs_enum_values,
            docs_i64_values,
            original_doc_id_counter: doc_id_counter,
        }
    }

    fn sort_and_merge_block_doclengths(&mut self) {
        self.all_block_doc_lengths.sort();

        self.doc_infos.extend(
            std::mem::take(&mut self.all_block_doc_lengths)
                .into_iter()
                .flat_map(|block_doc_lengths| block_doc_lengths.0),
        );
    }

    fn calculate_field_average_lengths(
        &mut self,
        writer: &mut Vec<u8>,
        num_docs: u32,
        num_scored_fields: usize,
        incremental_info: &mut IncrementalIndexInfo,
    ) {
        let mut total_field_lengths: Vec<u64> = vec![0; num_scored_fields];
        for (doc_id, worker_miner_doc_info) in self.doc_infos.iter().enumerate() {
            if !bitmap::check(&incremental_info.invalidation_vector, doc_id) {
                for (field_id, &field_length) in worker_miner_doc_info.field_lengths.iter().enumerate() {
                    total_field_lengths[field_id] += field_length as u64;
                }
            }
        }

        let num_docs = num_docs as f64;
        for (field_id, total_length) in total_field_lengths.into_iter().enumerate() {
            let average_length = self.average_lengths.get_mut(field_id).unwrap();
            *average_length = total_length as f64 / num_docs;
            writer.write_all(&(*average_length).to_le_bytes()).unwrap();
        }
    }

    /// Enum storage format:
    /// 4 bytes - N number of enum fields
    /// N enum fields bytes - store number of bits to encode the enum value ids,
    /// 4 bytes - u32 of X, for getting the entire slice easily
    /// X bits  - of bitpacked enum values. Stored per document, then per enum.
    /// 
    /// Returns a nested array of the enum value strings.
    /// Sorted according to enum_id and ev_id.
    fn write_enums(&mut self, field_infos: &Arc<FieldInfos>, doc_info_writer: &mut Vec<u8>) -> Vec<Vec<String>> {
        let num_enum_fields = field_infos.num_enum_fields;

        let mut enums_ev_id: Vec<EnumMax> = vec![0; num_enum_fields];
        let mut enums_ev_str_and_ids: Vec<FxHashMap<&str, EnumMax>> = vec![FxHashMap::default(); num_enum_fields];

        // -----------------------------------------------------
        // First repopulate incremental indexing info
        for field_info in field_infos.field_infos_by_id.iter() {
            if let Some(EnumInfo { enum_id, enum_values }) = &field_info.enum_info {
                enums_ev_id[*enum_id] = enum_values.len() as EnumMax;
                enums_ev_str_and_ids[*enum_id] = FxHashMap::from_iter(
                    enum_values.iter()
                        .enumerate()
                        .map(|(i, s)| (s.as_str(), i as u8 + 1))
                );
            }
        }
        // -----------------------------------------------------

        // -----------------------------------------------------
        // Next, assign ev_id (enum value ids) to the enums' values
        for worker_miner_doc_info in self.doc_infos.iter() {
            for (enum_id, doc_enum) in worker_miner_doc_info.doc_enums.iter().enumerate() {
                if doc_enum.is_empty() {
                    // Empty strings will be treated as non-existent
                    continue;
                }

                let enum_ev_str_and_ids: &mut FxHashMap<&str, u8> = unsafe {
                    enums_ev_str_and_ids.get_unchecked_mut(enum_id)
                };

                if !enum_ev_str_and_ids.contains_key(doc_enum.as_str()) {
                    let ev_id = unsafe { enums_ev_id.get_unchecked_mut(enum_id) };
                    if *ev_id < EnumMax::MAX {
                        // Start assigning from 1, 0 is the default ev (enum value)
                        // 0 being the default value facilitates incremental indexing
                        *ev_id += 1;
                        enum_ev_str_and_ids.insert(doc_enum, *ev_id);
                    }
                }
            }
        }
        // -----------------------------------------------------

        // -----------------------------------------------------
        // Calculate the minimum bits to represent the largest ev_id

        let mut capacity_per_document = 0;
        let enum_bit_starts: Vec<usize> = enums_ev_id.iter()
            .enumerate()
            .map(|(enum_id, &last_ev_id)| {
                if last_ev_id == EnumMax::MAX {
                    // Warn if enum values > EnumMax::MAX
                    let field_name = field_infos.field_infos_by_name.iter()
                        .find_map(|(field_name, field_info)| {
                            if let Some(EnumInfo { enum_id: curr_enum_id, enum_values: _ }) = &field_info.enum_info {
                                if *curr_enum_id == enum_id {
                                    return Some(field_name);
                                }
                            }
                            None
                        })
                        .unwrap();

                    warn!(
                        "More than {} enum values detected for field {}, excess values will be ignored",
                        EnumMax::MAX, field_name,
                    );
                }

                let num_bits = (last_ev_id as f64).log2() as usize + 1;
                capacity_per_document += num_bits;

                debug_assert!(num_bits <= 8);
                i_debug!("{} bits to store enum {}", num_bits, enum_id);

                8 - num_bits
            })
            .collect();
        // -----------------------------------------------------

        // -----------------------------------------------------
        // Then bitpack it

        let mut bitpacked_enum_values: BitVec<u8, Msb0> = BitVec::with_capacity(
            self.original_doc_id_counter as usize * capacity_per_document, 
        );

        // Previous documents
        for doc_id in 0..self.original_doc_id_counter as usize {
            for enum_id in 0..num_enum_fields {
                let idx = num_enum_fields * doc_id + enum_id;
                debug_assert!(enum_id < enum_bit_starts.len());
                debug_assert!(idx < self.docs_enum_values.len());

                let bit_start = unsafe { *enum_bit_starts.get_unchecked(enum_id) };
                let ev_id = unsafe { *self.docs_enum_values.get_unchecked(idx) };

                bitpacked_enum_values.extend_from_bitslice(&ev_id.view_bits::<Msb0>()[bit_start..]);
            }
        }

        // Current documents
        for worker_miner_doc_info in self.doc_infos.iter() {
            for (enum_id, doc_enum) in worker_miner_doc_info.doc_enums.iter().enumerate() {
                debug_assert!(enum_id < enum_bit_starts.len());
                let bit_start = unsafe { *enum_bit_starts.get_unchecked(enum_id) };
                let ev_id = if let Some(&ev_id) = unsafe {
                    enums_ev_str_and_ids.get_unchecked(enum_id).get(doc_enum.as_str())
                } {
                    ev_id
                } else {
                    0 as EnumMax
                };

                bitpacked_enum_values.extend_from_bitslice(&ev_id.view_bits::<Msb0>()[bit_start..]);
            }
        }
        // -----------------------------------------------------

        // -----------------------------------------------------
        // Flush misc info and the bitpacked slice

        doc_info_writer.write_all(&(num_enum_fields as u32).to_le_bytes()).unwrap();

        for num_bits in enum_bit_starts {
            doc_info_writer.write_all(&[(8 - num_bits) as u8]).unwrap();
        }

        doc_info_writer.write_all(bitpacked_enum_values.as_raw_slice()).unwrap();
        // -----------------------------------------------------

        // Sort the ev_str_and_ids by the id portion, return just the strings
        // for serialization into the output infi_search.json
        enums_ev_str_and_ids.into_iter()
            .map(|enum_ev_str_and_ids| {
                let mut as_vec: Vec<_> =  enum_ev_str_and_ids.into_iter().collect();
                as_vec.sort_by_key(|&(_, id)| id);
                as_vec.into_iter().map(|(str, _)| str.to_owned()).collect()
            })
            .collect()
    }

    fn write_nums(&mut self, field_infos: &Arc<FieldInfos>, doc_info_writer: &mut Vec<u8>) {
        let mut field_infos_i64: Vec<_> = field_infos.field_infos_by_id.iter()
            .filter(|fi| fi.i64_info.is_some())
            .collect();
        field_infos_i64.sort_by_key(|fi| fi.i64_info.as_ref().unwrap().id);

        doc_info_writer.write_all(&(field_infos.num_i64_fields as u32).to_le_bytes()).unwrap();

        // -----------------------------------------------------
        // Find the minimum
        let mut minimums = vec![i64::MAX; field_infos.num_i64_fields];

        // Old values
        if field_infos.num_i64_fields > 0 {
            // .chunks panics if == 0
            for chunk in self.docs_i64_values.chunks(field_infos.num_i64_fields) {
                for (idx, &v) in chunk.into_iter().enumerate() {
                    minimums[idx] = minimums[idx].min(v);
                }
            }
        }

        // New values
        for doc_info in self.doc_infos.iter() {
            for (idx, num) in doc_info.doc_nums.iter().enumerate() {
                let default = field_infos_i64[idx].i64_info.as_ref().unwrap().default;
                minimums[idx] = minimums[idx].min(num.unwrap_or(default));
            }
        }

        for min in minimums.iter() {
            doc_info_writer.write_all(&min.to_le_bytes()).unwrap();
        }
        // -----------------------------------------------------

        // -----------------------------------------------------
        // Write old values
        if field_infos.num_i64_fields > 0 {
            for chunk in self.docs_i64_values.chunks(field_infos.num_i64_fields) {
                for (idx, &v) in chunk.into_iter().enumerate() {
                    debug_assert!(v >= minimums[idx]);
                    let delta = v - minimums[idx];
                    debug_assert!(delta >= 0);
                    varint::get_var_int_vec_u64(delta as u64, doc_info_writer);
                }
            }
        }
        // -----------------------------------------------------

        // -----------------------------------------------------
        // Write new values
        for doc_info in self.doc_infos.iter() {
            for (idx, num) in doc_info.doc_nums.iter().enumerate() {
                let default = field_infos_i64[idx].i64_info.as_ref().unwrap().default;
                let num = num.unwrap_or(default);
                
                debug_assert!(num >= minimums[idx]);
                let delta = num - minimums[idx];
                debug_assert!(delta >= 0);

                varint::get_var_int_vec_u64(delta as u64, doc_info_writer);
            }
        }
        // -----------------------------------------------------
    }

    /// 4 bytes - number of documents
    /// 4 bytes - doc id counter
    /// 8 * Number of fields bytes - average field lengths
    /// 4 bytes - X + 4
    /// X bytes - of variable integers of field lengths
    /// Y bytes - from write_enums function
    /// 
    /// Returns:
    /// - Serialized document infos
    /// - Enum String to id mappings, for storing in the output infi_search.json
    pub fn finalize_and_flush(
        &mut self,
        num_docs: u32,
        field_infos: &Arc<FieldInfos>,
        incremental_info: &mut IncrementalIndexInfo,
    ) -> (Vec<u8>, Vec<Vec<String>>) {
        let num_scored_fields = field_infos.num_scored_fields;

        let mut doc_info_writer = Vec::with_capacity(
            8 + num_scored_fields * 8 + num_scored_fields * self.doc_infos.len()
        );

        self.sort_and_merge_block_doclengths();

        doc_info_writer.write_all(&num_docs.to_le_bytes()).unwrap();
        
        let doc_lengths_len = self.doc_infos.len() as u32;
        doc_info_writer.write_all(&doc_lengths_len.to_le_bytes()).unwrap();

        self.calculate_field_average_lengths(&mut doc_info_writer, num_docs, num_scored_fields, incremental_info);

        let mut field_length_writer = Vec::with_capacity(num_scored_fields * self.doc_infos.len());
        for worker_miner_doc_info in self.doc_infos.iter() {
            for &field_length in worker_miner_doc_info.field_lengths.iter() {
                varint::get_var_int_vec(field_length, &mut field_length_writer);
            }
        }

        doc_info_writer.write_all(&(field_length_writer.len() as u32 + 4).to_le_bytes()).unwrap();
        doc_info_writer.extend(field_length_writer);

        let enums_ev_str_and_ids = self.write_enums(field_infos, &mut doc_info_writer);
        self.write_nums(field_infos, &mut doc_info_writer);

        doc_info_writer.flush().unwrap();

        (doc_info_writer, enums_ev_str_and_ids)
    }
}

impl Default for DocInfos {
    fn default() -> Self {
        DocInfos {
            doc_infos: Vec::new(),
            all_block_doc_lengths: Vec::new(),
            average_lengths: vec![0.0; 0],
            docs_enum_values: Vec::new(),
            docs_i64_values: Vec::new(),
            original_doc_id_counter: 0,
        }
    }
}
