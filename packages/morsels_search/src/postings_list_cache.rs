const CACHE_SIZE: usize = 8;

/// LRU cache of postings lists, to prevent unnecessary repeated loading from disk / network
pub struct PostingsListCache {
    cache: Vec<(u32, Vec<u8>)>
}

impl PostingsListCache {
    pub fn new() -> PostingsListCache {
        PostingsListCache { cache: Vec::with_capacity(CACHE_SIZE) }
    }

    pub fn add(&mut self, pl: u32, postings_list: Vec<u8>) {
        if self.cache.iter().any(|(pl_num, _)| *pl_num == pl) {
            return;
        }

        if self.cache.len() == CACHE_SIZE {
            self.cache.remove(0);
        }

        self.cache.push((pl, postings_list));
    }

    pub fn get(&self, pl: u32) -> Option<&Vec<u8>> {
        self.cache.iter().find_map(|(pl_num, pl_vec)| {
            if *pl_num == pl {
                Some(pl_vec)
            } else {
                None
            }
        })
    }
}
