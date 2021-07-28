use byteorder::{ByteOrder, LittleEndian};

use web_sys::Response;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::JsCast;
use web_sys::Request;
use wasm_bindgen::JsValue;

pub struct DocInfo {
    pub doc_length_factors: Vec<Vec<f64>>,
    pub num_docs: u32,
}

impl DocInfo {
    pub async fn create(url: String, num_fields: usize) -> Result<DocInfo, JsValue> {
        let window: web_sys::Window = js_sys::global().unchecked_into();
        
        let doc_info_request = Request::new_with_str(&(url.clone() + "/docInfo"))?;
        let doc_info_fetch_future = JsFuture::from(window.fetch_with_request(&doc_info_request));
        let doc_info_resp_value = doc_info_fetch_future.await?;
        let doc_info_resp: Response = doc_info_resp_value.dyn_into().unwrap();
        let doc_info_array_buffer = JsFuture::from(doc_info_resp.array_buffer()?).await?;

        let doc_info_typebuf: js_sys::Uint8Array = js_sys::Uint8Array::new(&doc_info_array_buffer);
        let doc_info_vec: Vec<u8> = doc_info_typebuf.to_vec();

        let mut byte_offset = 0;
        let num_docs = LittleEndian::read_u32(&doc_info_vec);
        byte_offset += 4;
        let mut avg_doc_lengths: Vec<f64> = Vec::new();
        let mut doc_length_factors: Vec<Vec<f64>> = Vec::new();

        for _i in 0..num_fields {
            avg_doc_lengths.push(LittleEndian::read_u32(&doc_info_vec[byte_offset..]) as f64);
            byte_offset += 4;
        }

        let total_bytes = doc_info_vec.len();
        while byte_offset < total_bytes {
            let mut doc_field_lengths: Vec<f64> = Vec::with_capacity(num_fields as usize);
            for i in 0..num_fields {
                let field_length = LittleEndian::read_u32(&doc_info_vec[byte_offset..]) as f64;
                doc_field_lengths.push(field_length / avg_doc_lengths[i]);
                byte_offset += 4;
            }
            doc_length_factors.push(doc_field_lengths);
        }

        Ok(DocInfo {
            doc_length_factors,
            num_docs,
        })
    }
}
