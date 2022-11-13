mod dictionary_writer;
mod doc_info;
mod incremental_info;
pub mod indexer;
mod field_info;
mod loader;
mod spimi_reader;
mod spimi_writer;
mod utils;
mod worker;

#[macro_use]
extern crate lazy_static;

pub const INFISEARCH_VER: &str = env!("CARGO_PKG_VERSION");
pub static OLD_SOURCE_CONFIG: &str = "_old_config.json";
pub static OUTPUT_CONFIG_FILE: &str = "output_config.json"; // Jekyll ignores leading _ files
pub static SOURCE_CONFIG_FILE: &str = "infi_search.json";

pub use utils::assets;
