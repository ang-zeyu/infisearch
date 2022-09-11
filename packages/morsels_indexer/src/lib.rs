mod dictionary_writer;
mod docinfo;
mod incremental_info;
pub mod indexer;
pub mod fieldinfo;
pub mod loader;
mod spimireader;
mod spimiwriter;
mod utils;
mod worker;

#[macro_use]
extern crate lazy_static;

pub const MORSELS_VERSION: &str = env!("CARGO_PKG_VERSION");

