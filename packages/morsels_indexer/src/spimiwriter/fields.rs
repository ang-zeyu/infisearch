use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::Arc;

use crate::worker::miner::WorkerMinerDocInfo;
use crate::FieldInfos;


pub fn store_fields(
    check_for_existing_field_store: bool,
    start_doc_id: u32,
    field_infos: &Arc<FieldInfos>,
    doc_id_counter: u32,
    spimi_counter: u32,
    num_stores_per_dir: u32,
    block_number: u32,
    sorted_doc_infos: &mut Vec<WorkerMinerDocInfo>
) {
    let mut file_number = if check_for_existing_field_store {
        start_doc_id / field_infos.field_store_block_size
    } else {
        (doc_id_counter - spimi_counter) / field_infos.field_store_block_size
    };
    let mut curr_block_count = if check_for_existing_field_store {
        start_doc_id % field_infos.field_store_block_size
    } else {
        (doc_id_counter - spimi_counter) % field_infos.field_store_block_size
    };
    let mut writer = open_new_block_file(file_number, field_infos, num_stores_per_dir, block_number, check_for_existing_field_store);
    write_field_texts(
        &mut writer,
        sorted_doc_infos.first_mut().unwrap(),
        &mut curr_block_count,
        field_infos,
        &mut file_number,
    );
    for worker_miner_doc_info in sorted_doc_infos.iter_mut().skip(1) {
        if curr_block_count == 0 {
            writer = open_new_block_file(file_number, field_infos, num_stores_per_dir, block_number, check_for_existing_field_store);
        } else {
            writer.write_all(b",").unwrap();
        }
    
        write_field_texts(
            &mut writer,
            worker_miner_doc_info,
            &mut curr_block_count,
            field_infos,
            &mut file_number,
        );
    }
    if curr_block_count != 0 {
        writer.write_all(b"]").unwrap();
        writer.flush().unwrap();
    }
}

#[inline(always)]
fn open_new_block_file(
    file_number: u32,
    field_infos: &Arc<FieldInfos>,
    num_stores_per_dir: u32,
    block_number: u32,
    check_for_existing: bool,
) -> BufWriter<File> {
    let output_dir = field_infos.field_output_folder_path.join(
        (file_number / num_stores_per_dir).to_string()
    );
    if (file_number % num_stores_per_dir == 0)
        && !(output_dir.exists() && output_dir.is_dir())
    {
        std::fs::create_dir(&output_dir)
            .expect("Failed to create field store output dir!");
    }
    let output_file_path = output_dir.join(format!("{}--{}.json", file_number, block_number));
    if check_for_existing && output_file_path.exists() {
        // The first block for dynamic indexing might have been left halfway through somewhere before
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

        BufWriter::new(field_store_file)
    } else {
        let mut writer = BufWriter::new(
            File::create(output_file_path).expect("Failed to open field store for writing."),
        );
        writer.write_all(b"[").unwrap();
        writer
    }
}

#[inline(always)]
fn write_field_texts(
    writer: &mut BufWriter<File>,
    worker_miner_doc_info: &mut WorkerMinerDocInfo,
    curr_block_count: &mut u32,
    field_infos: &Arc<FieldInfos>,
    file_number: &mut u32,
) {
    writer.write_all(&std::mem::take(&mut worker_miner_doc_info.field_texts)).unwrap();
    *curr_block_count += 1;
    if *curr_block_count == field_infos.field_store_block_size {
        writer.write_all(b"]").unwrap();
        writer.flush().unwrap();
    
        *file_number += 1;
        *curr_block_count = 0;
    }
}
