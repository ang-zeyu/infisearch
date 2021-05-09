use std::sync::Mutex;
use std::collections::HashMap;
use std::io::BufWriter;
use std::path::Path;
use std::io::Write;
use std::fs::File;
use std::convert::TryInto;

use dashmap::DashMap;
use dashmap::mapref::multiple::RefMutMulti;

use crate::utils::varint::get_var_int;

static PREFIX_FRONT_CODE: u8 = 123;
static SUBSEQUENT_FRONT_CODE: u8 = 125;

pub struct DictionaryEntry {
    pub assigned_id: u32,
    pub doc_freq: u32,
    pub postings_file_name: u32,
    pub postings_file_offset: u16,
}

pub struct Dictionary<'a> {
    pub entries: DashMap<String, DictionaryEntry>,
    pub id_to_term_map: DashMap<u32, String>,
    pub next_term_id: Mutex<u32>,
    pub sorted_entries_by_term: Vec<RefMutMulti<'a, String, DictionaryEntry>>,
}

unsafe impl<'a> Send for Dictionary<'a> {}
unsafe impl<'a> Sync for Dictionary<'a> {}

impl<'a> Dictionary<'a> {
    pub fn add_term(&self, term: String) -> u32 {
        self.entries.entry(term.clone()).or_insert_with(|| {
            let mut next_id = self.next_term_id.lock().unwrap();

            let dict_entry = DictionaryEntry {
                assigned_id: *next_id,
                doc_freq: 0,
                postings_file_name: 0,
                postings_file_offset: 0
            };
        
            self.id_to_term_map.insert(*next_id, term);

            *next_id += 1;

            dict_entry
        }).assigned_id
    }

    pub fn sort_entries_by_term(&'a mut self) {
        self.sorted_entries_by_term = self.entries.iter_mut().collect();
        self.sorted_entries_by_term.sort_by(|a, b| a.key().cmp(b.key()));
    }

    fn dump_dict_table(& self, folder_path: &str) {
        let output_file_path = Path::new(folder_path).join("dictionaryTable");

        let f = File::create(output_file_path).expect("Failed to open dictionary table for writing.");
        let mut buffered_writer = BufWriter::new(f);
        
        let mut prev_postings_file_name = 0;
        for ref_mut_multi in self.sorted_entries_by_term.iter() {
            let dict_entry = ref_mut_multi.value();
            let difference: u8 = (dict_entry.postings_file_name - prev_postings_file_name).try_into().unwrap();
            buffered_writer.write_all(&[difference]).unwrap();
            prev_postings_file_name = dict_entry.postings_file_name;
            
            buffered_writer.write_all(&&get_var_int(dict_entry.doc_freq)).unwrap();

            buffered_writer.write_all(&dict_entry.postings_file_offset.to_le_bytes()).unwrap();
        }

        buffered_writer.flush().unwrap();
    }

    fn get_common_prefix_len(str1: &str, str2: &str) -> usize {
        let mut len = 0;

        while len < str1.len() && len < str2.len()
            && str1.chars().nth(len).unwrap() == str2.chars().nth(len).unwrap() {
            len += 1;
        }

        len
    }

    fn dump_daas(& self, folder_path: &str) {
        let output_file_path = Path::new(folder_path).join("dictionaryString");

        let f = File::create(output_file_path).expect("Failed to open dictionary string for writing.");
        let mut buffered_writer = BufWriter::new(f);

        //let borrowed_entry_iterator = &mut(&mut self.entries).into_iter();
        let sorted_entries: Vec<&String> = self.sorted_entries_by_term.iter().map(|ref_mut_multi| ref_mut_multi.key()).collect();

        for mut i in 0..sorted_entries.len() {
            let mut curr_common_prefix: &str = &sorted_entries[i];
            let mut num_frontcoded_terms = 0;
        
            let mut j = i + 1;
            while j < sorted_entries.len() {
                let common_prefix_len = Dictionary::get_common_prefix_len(curr_common_prefix, &sorted_entries[j]);
                if common_prefix_len <= 2 {
                break;
                }
        
                if common_prefix_len < curr_common_prefix.len() {
                if common_prefix_len == curr_common_prefix.len() - 1 {
                    // equally worth it
                    curr_common_prefix = &curr_common_prefix[..common_prefix_len];
                } else {
                    // not worth it
                    break;
                }
                }
        
                num_frontcoded_terms += 1;
                j += 1;
            }
        
            let term_buffer = sorted_entries[i].as_bytes();
            let curr_prefix_buffer = curr_common_prefix.as_bytes();
            buffered_writer.write_all(&[term_buffer.len().try_into().unwrap()]).unwrap();
            buffered_writer.write_all(curr_prefix_buffer).unwrap();
            if num_frontcoded_terms > 0 {
                buffered_writer.write_all(&[PREFIX_FRONT_CODE]).unwrap();
                buffered_writer.write_all(&term_buffer[curr_prefix_buffer.len()..]).unwrap();
            }
        
            while num_frontcoded_terms > 0 {
                num_frontcoded_terms -= 1;
                i += 1;
                let frontcoded_term_buffer = sorted_entries[i].as_bytes();
                buffered_writer.write_all(&[(frontcoded_term_buffer.len() - curr_prefix_buffer.len()).try_into().unwrap()]).unwrap();
                buffered_writer.write_all(&[SUBSEQUENT_FRONT_CODE]).unwrap();
                buffered_writer.write_all(&frontcoded_term_buffer[curr_prefix_buffer.len()..]).unwrap();
            }
        }

        buffered_writer.flush().unwrap();
    }

    pub fn dump(& self, folder_path: &str) {
        self.dump_dict_table(folder_path);
        self.dump_daas(folder_path);
    }
}
