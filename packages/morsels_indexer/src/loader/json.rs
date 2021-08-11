use std::path::Path;

use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde_json::value::Value;

use crate::loader::BasicLoaderResult;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;

#[derive(Deserialize)]
pub struct JsonLoaderOptions {
    #[serde(default = "FxHashMap::default")]
    field_map: FxHashMap<String, String>,
    #[serde(default = "Vec::new")]
    field_order: Vec<String>,
}

pub struct JsonLoader {
    options: JsonLoaderOptions,
}

impl JsonLoader {
    pub fn get_new_json_loader(config: serde_json::Value) -> Box<Self> {
        let json_loader_options: JsonLoaderOptions = serde_json::from_value(config).expect("JsonLoader options did not match schema!");

        Box::new(JsonLoader {
            options: json_loader_options,
        })
    }

    fn unwrap_json_deserialize_result (
        &self,
        mut read_result: FxHashMap<String, String>
    ) -> Box<dyn LoaderResult + Send> {
        let mut field_texts: Vec<(String, String)> = Vec::with_capacity(self.options.field_order.len());

        for header_name in self.options.field_order.iter() {
            if let Some((field_name, text)) = read_result.remove_entry(header_name) {
                field_texts.push((
                    self.options.field_map.get(&field_name).unwrap().to_owned(),
                    text,
                ));
            }
        }

        Box::new(BasicLoaderResult {
            field_texts,
        }) as Box<dyn LoaderResult + Send>
    }
}

impl Loader for JsonLoader {
    fn try_index_file<'a> (&'a self, _input_folder_path: &Path, path: &Path) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = path.extension() {
            if extension == "json" {
                let as_value: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("Failed to read json file!")).expect("Invalid json!");

                if as_value.is_array() {
                    let documents: Vec<FxHashMap<String, String>> = serde_json::from_value(as_value).unwrap();
                    return Some(Box::new(documents.into_iter().map(move |document| {
                        self.unwrap_json_deserialize_result(document)
                    })));
                } else {
                    return Some(Box::new(std::iter::once(
                        self.unwrap_json_deserialize_result(serde_json::from_value(as_value).unwrap())
                    )));
                }
            }
        }

        None
    }
}
