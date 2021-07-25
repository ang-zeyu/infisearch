mod dictionary;
mod docinfo;
mod PostingsList;
mod Searcher;
mod utils;

use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

use librarian_common::tokenize;

#[wasm_bindgen]
pub fn wasm_tokenize(text: String) -> JsValue {
    JsValue::from_serde(&tokenize::english::wasm_tokenize(text)).unwrap()
}

#[wasm_bindgen]
pub fn get_stop_words() -> String {
   tokenize::english::get_stop_words().to_owned()
}
