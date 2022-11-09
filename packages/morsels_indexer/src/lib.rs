mod dictionary_writer;
mod docinfo;
mod incremental_info;
pub mod indexer;
mod fieldinfo;
mod loader;
mod spimireader;
mod spimiwriter;
mod utils;
mod worker;

#[macro_use]
extern crate lazy_static;

pub const MORSELS_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static OLD_MORSELS_CONFIG: &str = "_old_config.json";

pub use utils::assets;
