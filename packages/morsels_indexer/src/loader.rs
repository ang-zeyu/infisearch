pub mod csv;
pub mod html;
pub mod json;

use std::path::Path;

pub type LoaderResultIterator<'a> = Box<dyn Iterator<Item = Box<dyn LoaderResult + Send>> + 'a>;

pub trait Loader {
    fn try_index_file(&self, input_folder_path: &Path, path: &Path) -> Option<LoaderResultIterator>;
}

pub trait LoaderResult {
    fn get_field_texts(&mut self) -> Vec<(String, String)>;
}

pub struct BasicLoaderResult {
    field_texts: Vec<(String, String)>,
}

impl LoaderResult for BasicLoaderResult {
    fn get_field_texts(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.field_texts)
    }
}
