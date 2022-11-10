pub mod ascii_folding_filter;
pub mod ascii;
pub mod spelling;
pub mod stop_words;
pub mod utils;

#[macro_use]
#[cfg(feature = "indexer")]
extern crate lazy_static;
