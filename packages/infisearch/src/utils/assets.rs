use std::fs::File;
use std::io::Write;
use std::path::Path;

use include_dir::{include_dir, Dir};

const SEARCH_UI_DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/search-ui-dist");

pub fn write_infisearch_assets(assets_output_dir: &Path) {
    std::fs::create_dir_all(assets_output_dir)
        .expect("Failed to create assets output directory");
    for file in SEARCH_UI_DIST.files() {
        if let Some(file_ext) = file.path().extension() {
            if ["css", "js", "wasm"].iter().any(|&ext| ext == file_ext) {
                let mut output_file = File::create((assets_output_dir).join(file.path()))
                    .expect("Failed to open asset write handler");
                output_file.write_all(file.contents()).expect("Failed to copy assets!");
            }
        }
    }
}
