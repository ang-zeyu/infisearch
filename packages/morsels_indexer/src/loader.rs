pub mod csv;
pub mod html;
pub mod json;
pub mod pdf;
pub mod txt;

use std::path::{Path, PathBuf};

use crate::worker::miner::Zone;

pub type LoaderResultIterator<'a> = Box<dyn Iterator<Item = Box<dyn LoaderResult + Send>> + 'a>;

pub type LoaderBoxed = Box<dyn Loader + Send + Sync>;

#[typetag::serde(tag = "type")]
pub trait Loader {
    fn try_index_file(
        &self,
        absolute_path: &Path,
        relative_path: &Path,
    ) -> Option<LoaderResultIterator>;

    fn get_name(&self) -> String;
}

pub trait LoaderResult {
    fn get_field_texts_and_path(self: Box<Self>) -> (Vec<Zone>, PathBuf);
}

pub struct BasicLoaderResult {
    field_texts: Vec<Zone>,
    absolute_path: PathBuf,
}

impl LoaderResult for BasicLoaderResult {
    fn get_field_texts_and_path(self: Box<Self>) -> (Vec<Zone>, PathBuf) {
        (self.field_texts, self.absolute_path)
    }
}
