use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

use crate::field_info::FieldInfos;
use crate::utils::reusable_writer::ReusableWriter;


#[allow(clippy::too_many_arguments)]
pub fn store_fields(
    check_for_existing_field_store: bool,
    start_doc_id: u32,
    field_infos: &Arc<FieldInfos>,
    doc_id_counter: u32,
    spimi_counter: u32,
    block_number: u32,
    field_texts: Vec<Vec<u8>>,
) {
    let file_number = if check_for_existing_field_store {
        start_doc_id / field_infos.num_docs_per_store
    } else {
        (doc_id_counter - spimi_counter) / field_infos.num_docs_per_store
    };
    let end_file_number = if check_for_existing_field_store {
        (start_doc_id + field_texts.len() as u32 - 1) / field_infos.num_docs_per_store
    } else {
        (doc_id_counter - spimi_counter + field_texts.len() as u32 - 1) / field_infos.num_docs_per_store
    };

    let block_number = block_number.to_string();
    let mut prev_subdir = std::u32::MAX;
    for file_number in file_number..(end_file_number + 1) {
        let subdir = file_number / field_infos.num_stores_per_dir;
        if subdir == prev_subdir {
            continue;
        }

        prev_subdir = subdir;

        let output_dir = field_infos.field_output_folder_path.join(subdir.to_string());
        if !output_dir.is_dir() {
            std::fs::create_dir(&output_dir).expect("Failed to create field store output dir!");
        }

        let output_dir = output_dir.join(&block_number);
        if !output_dir.is_dir() {
            std::fs::create_dir(&output_dir).expect("Failed to create field store output block dir!");
        }
    }

    let mut file_number = file_number;
    let mut curr_block_count = if check_for_existing_field_store {
        start_doc_id % field_infos.num_docs_per_store
    } else {
        (doc_id_counter - spimi_counter) % field_infos.num_docs_per_store
    };

    let mut writer = ReusableWriter::new();

    open_new_block_file(
        &mut writer,
        field_infos,
        file_number,
        &block_number,
        check_for_existing_field_store,
    );
    write_field_texts(
        &mut writer,
        unsafe { field_texts.first().unwrap_unchecked() },
        &mut curr_block_count,
        field_infos,
        &mut file_number,
    );
    for field_texts in field_texts.into_iter().skip(1) {
        if curr_block_count == 0 {
            open_new_block_file(
                &mut writer, 
                field_infos,
                file_number,
                &block_number,
                check_for_existing_field_store,
            );
        } else {
            writer.write(b",");
        }
    
        write_field_texts(
            &mut writer,
            &field_texts,
            &mut curr_block_count,
            field_infos,
            &mut file_number,
        );
    }
    if curr_block_count != 0 {
        writer.write(b"]");
        writer.flush();
    }
}

#[inline(always)]
fn open_new_block_file(
    buf_writer: &mut ReusableWriter,
    field_infos: &Arc<FieldInfos>,
    file_number: u32,
    block_number: &str,
    check_for_existing: bool,
) {
    let output_file_path = field_infos.field_output_folder_path
        .join((file_number / field_infos.num_stores_per_dir).to_string())
        .join(block_number)
        .join(format!("{}.json", file_number));

    if check_for_existing && output_file_path.exists() {
        // The first block for incremental indexing might have been left halfway through somewhere before
        let mut field_store_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(output_file_path)
            .expect("Failed to open existing field store for editing");
        field_store_file
            .seek(SeekFrom::End(-1))
            .expect("Failed to seek to existing field store end");

        // Override ']' with ','
        field_store_file
            .write_all(b",")
            .expect("Failed to override existing field store ] with ,");

        buf_writer.change_file(field_store_file);
    } else {
        buf_writer.change_file(
            File::create(output_file_path).expect("Failed to open field store for writing.")
        );
        buf_writer.write(b"[");
    }
}

#[inline(always)]
fn write_field_texts(
    writer: &mut ReusableWriter,
    field_texts: &Vec<u8>,
    curr_block_count: &mut u32,
    field_infos: &Arc<FieldInfos>,
    file_number: &mut u32,
) {
    writer.write(field_texts);
    *curr_block_count += 1;
    if *curr_block_count == field_infos.num_docs_per_store {
        writer.write(b"]");
        writer.flush();
    
        *file_number += 1;
        *curr_block_count = 0;
    }
}
