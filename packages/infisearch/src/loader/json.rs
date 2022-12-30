use std::path::Path;
use std::path::PathBuf;

use log::error;
use log::warn;
use path_slash::PathExt;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Map;
use serde_json::value::Value;

use crate::field_info::{ADD_FILES_FIELD, RELATIVE_FP_FIELD};
use crate::loader::BasicLoaderResult;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;
use crate::worker::miner::{DEFAULT_ZONE_SEPARATION, Zone};

#[derive(Serialize, Deserialize)]
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
        let mut json_loader_options: JsonLoaderOptions =
            serde_json::from_value(config).expect("JsonLoader options did not match schema!");

        if json_loader_options.field_order.is_empty() {
            let arbitrary_field_order: Vec<String> = json_loader_options.field_map
                .keys()
                .map(String::to_owned)
                .collect();
            json_loader_options.field_order = arbitrary_field_order;
        }

        json_loader_options.field_order.push(ADD_FILES_FIELD.to_owned());
        json_loader_options.field_map.insert(ADD_FILES_FIELD.to_owned(), ADD_FILES_FIELD.to_owned());

        Box::new(JsonLoader { options: json_loader_options })
    }

    fn unwrap_json_deserialize_result(
        &self,
        map: &Map<String, Value>,
        link: String,
        absolute_path: PathBuf,
    ) -> Box<dyn LoaderResult + Send> {
        let mut field_texts: Vec<Zone> = Vec::with_capacity(self.options.field_order.len() + 1);

        field_texts.push(Zone {
            field_name: RELATIVE_FP_FIELD.to_owned(),
            field_text: link,
            separation: DEFAULT_ZONE_SEPARATION,
        });

        for header_name in self.options.field_order.iter() {
            if let Some((field_name, value)) = map.get_key_value(header_name) {
                let field_text = if let Some(text) = value.as_str() {
                    text.to_owned()
                } else if let Some(int) = value.as_i64() {
                    int.to_string()
                } else if let Some(double) = value.as_f64() {
                    double.to_string()
                } else if value.is_null() {
                    continue;
                } else {
                    panic!(
                        "Invalid JSON value in array {}, expected String or Number or null.",
                        absolute_path.to_slash_lossy()
                    );
                };

                field_texts.push(Zone {
                    field_name: self.options.field_map.get(field_name)
                        .expect("field_order does not match field_map!")
                        .to_owned(),
                    field_text,
                    separation: DEFAULT_ZONE_SEPARATION,
                });
            }
        }

        Box::new(BasicLoaderResult { field_texts, absolute_path }) as Box<dyn LoaderResult + Send>
    }
}

impl Loader for JsonLoader {
    fn try_index_file<'a>(
        &'a self,
        absolute_path: &Path,
        relative_path: &Path,
    ) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = relative_path.extension() {
            if extension == "json" {
                let as_value: Value = serde_json::from_str(
                    &std::fs::read_to_string(absolute_path).expect("Failed to read json file!"),
                ).unwrap_or_else(|_err| panic!("Invalid json! {}", relative_path.to_slash_lossy()));

                let link = relative_path.to_slash();
                if link.is_none() {
                    error!("Unable to index {} containing non-unicode characters", relative_path.to_slash_lossy());
                    return None;
                }

                let link = unsafe { link.unwrap_unchecked().into_owned() };
                let absolute_path_as_buf = PathBuf::from(absolute_path);

                if let Some(values) = as_value.as_array() {
                    return Some(Box::new({
                        let doc_count = values.len();
                        let links = vec![link; doc_count];
                        values.to_owned()
                            .into_iter()
                            .zip(links)
                            .zip(0..doc_count)
                            .filter_map(move |((value, link), idx)|
                                if let Some(map) = value.as_object() {
                                    Some(self.unwrap_json_deserialize_result(
                                        map, format!("{}#{}", link, idx), absolute_path_as_buf.clone(),
                                    ))
                                } else {
                                    warn!(
                                        "Invalid JSON document in array {}, expected Map. Skipping.",
                                        link
                                    );
                                    None
                                }
                            )
                    }));
                } else if let Some(map) = as_value.as_object() {
                    return Some(Box::new(std::iter::once(self.unwrap_json_deserialize_result(
                        map, link, absolute_path_as_buf,
                    ))));
                } else {
                    warn!(
                        "Invalid JSON document {}, expected Map or Vec<Map>. Skipping.",
                        relative_path.to_slash_lossy()
                    )
                }
            }
        }

        None
    }

    fn get_name(&self) -> String {
        "JsonLoader".to_owned()
    }
}

impl Serialize for JsonLoader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.options.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JsonLoader {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("Called deserialize for JsonLoader")
    }
}
