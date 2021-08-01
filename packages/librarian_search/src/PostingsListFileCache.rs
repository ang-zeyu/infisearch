use futures::future::join_all;
use rustc_hash::FxHashMap;
use wasm_bindgen::JsCast;

use crate::PostingsList::PostingsList;

pub struct PostingsListFileCache {
    pl_bytes: FxHashMap<u32, Vec<u8>>,
}

impl PostingsListFileCache {
    pub async fn create(base_url: &str, pl_numbers: &Vec<u32>) -> PostingsListFileCache {
        let window: web_sys::Window = js_sys::global().unchecked_into();
        let pls = join_all(
            pl_numbers.iter().map(|pl_num| PostingsList::fetch_pl_to_vec(&window, base_url, *pl_num))
        ).await;

        let mut pl_bytes: FxHashMap<u32, Vec<u8>> = FxHashMap::default();
        for (idx, pl) in pls.into_iter().enumerate() {
            let pl_num = pl_numbers[idx];
            if let Ok(pl_vec) = pl {
                pl_bytes.insert(pl_num, pl_vec);
            }
        }

        PostingsListFileCache {
            pl_bytes,
        }
    }

    pub fn get(&self, pl_num: u32) -> Option<&Vec<u8>> {
        self.pl_bytes.get(&pl_num)
    }
}
