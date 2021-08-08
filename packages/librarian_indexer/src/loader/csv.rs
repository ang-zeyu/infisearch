use std::path::Path;

use csv::Reader;
use rustc_hash::FxHashMap;
use serde::Deserialize;

use crate::loader::BasicLoaderResult;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;

#[derive(Deserialize)]
pub struct CsvLoaderOptions {
    #[serde(default)]
    has_headers: bool,
    index_field_map: FxHashMap<usize, String>,
    header_field_map: FxHashMap<String, String>,
}

pub struct CsvLoader {
    pub options: CsvLoaderOptions,
}

impl CsvLoader {
    fn unwrap_csv_read_result<'a> (&'a self, read_result: Result<csv::StringRecord, csv::Error>, num_fields: usize) -> Box<dyn LoaderResult + Send> {
        let mut field_texts: Vec<(&'static str, String)> = Vec::with_capacity(num_fields);

        let record = read_result.expect("Failed to unwrap csv record result!");
        if self.options.has_headers {
            field_texts.push(("title", record[1].to_string()));
        } else {
            field_texts.push(("title", record[1].to_string()));
        }

        Box::new(BasicLoaderResult {
            field_texts,
        }) as Box<dyn LoaderResult + Send>
    }
}

impl Loader for CsvLoader {
    fn try_index_file<'a> (&'a self, _input_folder_path: &Path, path: &Path) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = path.extension() {
            if extension == "csv" {
                let num_fields = if self.options.has_headers {
                    self.options.header_field_map.len()
                } else {
                    self.options.index_field_map.len()
                };

                return Some(Box::new(
                    Reader::from_path(path).unwrap().into_records().map(move |result| {
                        self.unwrap_csv_read_result(result, num_fields)
                    })
                ));
            }
        }

        None
    }
}
