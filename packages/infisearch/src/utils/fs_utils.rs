use std::{fs, path::Path};

use log::warn;

pub fn clean_dir(folder_path: &Path) {
    if let Ok(read_dir) = fs::read_dir(folder_path) {
        for dir_entry in read_dir {
            if let Err(err) = dir_entry {
                warn!("Failed to clean {}, continuing.", err);
                continue;
            }
    
            let dir_entry = dir_entry.unwrap();
            let file_type = dir_entry.file_type();
            if let Err(err) = file_type {
                warn!("Failed to get file type when cleaning output dir {}, continuing.", err);
                continue;
            }
    
            let file_type = file_type.unwrap();
            if file_type.is_file() {
                if let Err(err) = fs::remove_file(dir_entry.path()) {
                    warn!("{}\nFailed to clean {}, continuing.", err, dir_entry.path().to_string_lossy());
                }
            } else if file_type.is_dir() {
                if let Err(err) = fs::remove_dir_all(dir_entry.path()) {
                    warn!("{}\nFailed to clean directory {}, continuing.", err, dir_entry.path().to_string_lossy());
                }
            }
        }
    } else {
        warn!("Failed to read output dir for cleaning, continuing.");
    }
}
