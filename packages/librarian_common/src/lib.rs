pub mod tokenize;

use wasm_bindgen::prelude::*;

#[macro_use]
extern crate lazy_static;

#[wasm_bindgen]
pub fn wasm_tokenize(text: String) -> JsValue {
    JsValue::from_serde(&tokenize::english::tokenize(text)).unwrap()
}

#[wasm_bindgen]
pub fn get_stop_words() -> String {
   tokenize::english::get_stop_words().to_owned()
}
