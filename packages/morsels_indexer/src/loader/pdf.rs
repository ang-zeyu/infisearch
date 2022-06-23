use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use log::error;
use path_slash::PathExt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;

fn default_field() -> String {
    "body".to_owned()
}

#[derive(Serialize, Deserialize)]
pub struct PdfLoaderOptions {
    #[serde(default = "default_field")]
    field: String,
}

pub struct PdfLoader {
    pub options: Arc<PdfLoaderOptions>,
}

struct PdfLoaderResult {
    link: String,
    absolute_path: PathBuf,
    options: Arc<PdfLoaderOptions>,
}

impl PdfLoader {
    pub fn get_new_pdf_loader(config: serde_json::Value) -> Box<Self> {
        let opts: PdfLoaderOptions = serde_json::from_value(config)
            .expect("PdfLoader options did not match schema!");

        Box::new(PdfLoader { options: Arc::new(opts) })
    }

    fn get_pdf_loader_result(&self, absolute_path: &Path, link: String) -> Box<dyn LoaderResult + Send> {
        Box::new(PdfLoaderResult {
            link,
            absolute_path: PathBuf::from(absolute_path),
            options: self.options.clone(),
        }) as Box<dyn LoaderResult + Send>
    }
}

#[typetag::serde]
impl Loader for PdfLoader {
    fn try_index_file<'a>(
        &'a self,
        absolute_path: &Path,
        relative_path: &Path,
    ) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = relative_path.extension() {
            if extension == "pdf" {
                let link = relative_path.to_slash().unwrap();

                return Some(Box::new(std::iter::once(
                    self.get_pdf_loader_result(absolute_path, link),
                )));
            }
        }

        None
    }

    fn get_name(&self) -> String {
        "PdfLoader".to_owned()
    }
}

impl Serialize for PdfLoader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.options.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PdfLoader {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("Called deserialize for PdfLoader")
    }
}

impl PdfLoaderResult {
    fn format_title(path: &Path, link: &str) -> String {
        if let Some(file_name) = path.file_name() {
            if let Some(file_name) = file_name.to_str() {
                return format!("{} (pdf)", &file_name[0..file_name.len() - 4]);
            }
        }

        link.to_owned()
    }
}

impl LoaderResult for PdfLoaderResult {
    fn get_field_texts(&mut self) -> Vec<(String, String)> {
        let title = Self::format_title(&self.absolute_path, &self.link);
        let text = if let Ok(text) = pdf_extract::extract_text(std::mem::take(&mut self.absolute_path)) {
            text
        } else {
            error!("Failed to parse pdf {}", &self.link);
            String::new()
        };

        vec![
            ("_relative_fp".to_owned(), std::mem::take(&mut self.link)),
            ("title".to_owned(), title),
            (self.options.field.clone(), text),
        ]
    }
}
