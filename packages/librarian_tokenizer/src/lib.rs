pub mod tokenize;

use wasm_bindgen::prelude::*;

#[macro_use]
extern crate lazy_static;

#[wasm_bindgen]
pub fn wasm_tokenize(text: String) -> JsValue {
    JsValue::from_serde(&tokenize::english::tokenize(text)).unwrap()
}
