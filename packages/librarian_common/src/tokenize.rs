pub mod english;

use serde::{Serialize};

#[derive(Serialize)]
pub struct WasmTokenizeResult {
    pub terms: Vec<String>,
    pub should_expand: bool,
}
