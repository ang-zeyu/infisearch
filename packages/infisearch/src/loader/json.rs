use std::path::Path;
use std::path::PathBuf;

use log::error;
use path_slash::PathExt;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

        json_loader_options.field_order.push(ADD_FILES_FIELD.to_owned());
        json_loader_options.field_map.insert(ADD_FILES_FIELD.to_owned(), ADD_FILES_FIELD.to_owned());

        Box::new(JsonLoader { options: json_loader_options })
    }

    fn unwrap_json_deserialize_result(
        &self,
        mut read_result: FxHashMap<String, String>,
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
            if let Some((field_name, text)) = read_result.remove_entry(header_name) {
                field_texts.push(Zone {
                    field_name: self.options.field_map.get(&field_name)
                        .expect("field_order does not match field_map!")
                        .to_owned(),
                    field_text: text,
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
                )
                .expect("Invalid json!");

                let link = relative_path.to_slash();
                if link.is_none() {
                    error!("Unable to index {} containing non-unicode characters", relative_path.to_slash_lossy());
                    return None;
                }

                let link = unsafe { link.unwrap_unchecked().into_owned() };
                let absolute_path_as_buf = PathBuf::from(absolute_path);

                if as_value.is_array() {
                    let documents: Vec<FxHashMap<String, String>> = serde_json::from_value(as_value)
                        .unwrap_or_else(|_| panic!(
                            "Json file {} not in the expected format of [{{ \"field_name\": \"... field text ...\", ... }}]!",
                            relative_path.as_os_str().to_string_lossy()
                        ));

                    return Some(Box::new({
                        let doc_count = documents.len();
                        let links = vec![link; doc_count];
                        documents.into_iter().zip(links).zip(0..doc_count).map(
                            move |((document, link), idx)| {
                                self.unwrap_json_deserialize_result(
                                    document, format!("{}#{}", link, idx), absolute_path_as_buf.clone(),
                                )
                            },
                        )
                    }));
                } else {
                    let document = serde_json::from_value(as_value)
                        .unwrap_or_else(|_| panic!(
                            "Json file {} not in the expected format of {{ \"field_name\": \"... field text ...\", ... }}!",
                            relative_path.as_os_str().to_string_lossy()
                        ));

                    return Some(Box::new(std::iter::once(self.unwrap_json_deserialize_result(
                        document, link, absolute_path_as_buf,
                    ))));
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
