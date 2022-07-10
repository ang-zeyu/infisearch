use std::path::Path;
use std::path::PathBuf;

use path_slash::PathExt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::fieldinfo::RELATIVE_FP_FIELD;
use crate::loader::BasicLoaderResult;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;
use crate::worker::miner::{DEFAULT_ZONE_SEPARATION, Zone};

#[derive(Serialize, Deserialize)]
pub struct TxtLoaderOptions {
    field: String,
}

pub struct TxtLoader {
    options: TxtLoaderOptions,
}

impl TxtLoader {
    pub fn get_new_txt_loader(config: serde_json::Value) -> Box<Self> {
        let json_loader_options: TxtLoaderOptions = serde_json::from_value(config)
            .expect("TxtLoader options did not match schema!");

        Box::new(TxtLoader { options: json_loader_options })
    }

    fn get_txt_loader_result(&self, text: String, link: String, absolute_path: PathBuf) -> Box<dyn LoaderResult + Send> {
        let field_texts = vec![
            Zone {
                field_name: RELATIVE_FP_FIELD.to_owned(),
                field_text: link,
                separation: DEFAULT_ZONE_SEPARATION,
            },
            Zone {
                field_name: self.options.field.clone(),
                field_text: text,
                separation: DEFAULT_ZONE_SEPARATION,
            },
        ];
        Box::new(BasicLoaderResult { field_texts, absolute_path }) as Box<dyn LoaderResult + Send>
    }
}

#[typetag::serde]
impl Loader for TxtLoader {
    fn try_index_file<'a>(
        &'a self,
        absolute_path: &Path,
        relative_path: &Path,
    ) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = relative_path.extension() {
            if extension == "txt" {
                let absolute_path_as_buf = PathBuf::from(absolute_path);
                let text = std::fs::read_to_string(absolute_path)
                    .unwrap_or_else(|_| panic!("Failed to read .txt file {}", absolute_path.to_string_lossy().into_owned()));
                let link = relative_path.to_slash().unwrap();
                return Some(Box::new(std::iter::once(
                    self.get_txt_loader_result(text, link, absolute_path_as_buf),
                )));
            }
        }

        None
    }

    fn get_name(&self) -> String {
        "TxtLoader".to_owned()
    }
}

impl Serialize for TxtLoader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.options.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TxtLoader {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("Called deserialize for TxtLoader")
    }
}
