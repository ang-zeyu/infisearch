use std::path::Path;

use csv::{ReaderBuilder};
use rustc_hash::FxHashMap;
use serde::Deserialize;

use crate::loader::BasicLoaderResult;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;

fn get_default_delimiter() -> u8 { b","[0] }

fn get_default_quote() -> u8 { b"\""[0] }

fn get_true() -> bool { true }

#[derive(Deserialize)]
struct CsvLoaderParseOptions {
    #[serde(default = "get_true")]
    has_headers: bool,
    #[serde(default = "get_default_delimiter")]
    delimiter: u8,
    #[serde(default = "get_default_quote")]
    quote: u8,
    #[serde(default = "get_true")]
    double_quote: bool,

    escape: Option<u8>,

    comment: Option<u8>,
}

impl Default for CsvLoaderParseOptions {
    fn default() -> Self {
        CsvLoaderParseOptions {
            has_headers: get_true(),
            delimiter: get_default_delimiter(),
            quote: get_default_quote(),
            double_quote: get_true(),
            escape: None,
            comment: None
        }
    }
}

#[derive(Deserialize)]
pub struct CsvLoaderOptions {
    #[serde(default = "CsvLoaderParseOptions::default")]
    parse_options: CsvLoaderParseOptions,
    #[serde(default)]
    use_headers: bool,
    #[serde(default = "FxHashMap::default")]
    index_field_map: FxHashMap<usize, String>,
    #[serde(default = "Vec::new")]
    index_field_order: Vec<usize>,
    #[serde(default = "FxHashMap::default")]
    header_field_map: FxHashMap<String, String>,
    #[serde(default = "Vec::new")]
    header_field_order: Vec<String>,
}

pub struct CsvLoader {
    options: CsvLoaderOptions,

    reader_builder: ReaderBuilder,
}

impl CsvLoader {
    pub fn get_new_csv_loader(config: serde_json::Value) -> Box<Self> {
        let csv_loader_options: CsvLoaderOptions = serde_json::from_value(config).expect("CsvLoader options did not match schema!");

        let csv_loader_parse_opts = &csv_loader_options.parse_options;
        let mut reader_builder = ReaderBuilder::new();
        reader_builder.has_headers(csv_loader_parse_opts.has_headers)
            .delimiter(csv_loader_parse_opts.delimiter)
            .quote(csv_loader_parse_opts.quote)
            .double_quote(csv_loader_parse_opts.double_quote)
            .escape(csv_loader_parse_opts.escape)
            .comment(csv_loader_parse_opts.comment);

        Box::new(CsvLoader {
            options: csv_loader_options,
            reader_builder,
        })
    }

    fn unwrap_csv_read_result (
        &self,
        read_result: Result<csv::StringRecord, csv::Error>,
        num_fields: usize,
    ) -> Box<dyn LoaderResult + Send> {
        let mut field_texts: Vec<(String, String)> = Vec::with_capacity(num_fields);

        let record = read_result.expect("Failed to unwrap csv record result!");
        for idx in self.options.index_field_order.iter() {
            if let Some(text) = record.get(*idx) {
                field_texts.push((
                    self.options.index_field_map.get(idx).unwrap().to_owned(),
                    text.to_owned(),
                ));
            }
        }

        Box::new(BasicLoaderResult {
            field_texts,
        }) as Box<dyn LoaderResult + Send>
    }

    fn unwrap_csv_deserialize_result (
        &self,
        read_result: FxHashMap<String, String>,
        num_fields: usize,
    ) -> Box<dyn LoaderResult + Send> {
        let mut field_texts: Vec<(String, String)> = Vec::with_capacity(num_fields);

        for header_name in self.options.header_field_order.iter() {
            if let Some(text) = read_result.get(header_name) {
                field_texts.push((
                    self.options.header_field_map.get(header_name).unwrap().to_owned(),
                    text.to_owned(),
                ));
            }
        }

        Box::new(BasicLoaderResult {
            field_texts,
        }) as Box<dyn LoaderResult + Send>
    }
}

impl Loader for CsvLoader {
    fn try_index_file<'a> (&'a self, _input_folder_path: &Path, absolute_path: &Path, relative_path: &Path) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = relative_path.extension() {
            if extension == "csv" {
                let num_fields = if self.options.use_headers {
                    self.options.header_field_map.len()
                } else {
                    self.options.index_field_map.len()
                };

                return Some(
                    if self.options.use_headers {
                        Box::new(self.reader_builder.from_path(absolute_path).unwrap().into_deserialize().map(move |result| {
                            self.unwrap_csv_deserialize_result(result.unwrap(), num_fields)
                        }))
                    } else {
                        Box::new(self.reader_builder.from_path(absolute_path).unwrap().into_records().map(move |result| {
                            self.unwrap_csv_read_result(result, num_fields)
                        }))
                    }
                );
            }
        }

        None
    }
}