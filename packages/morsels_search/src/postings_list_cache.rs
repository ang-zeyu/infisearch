const CACHE_SIZE: usize = 8;

// TODO make this configurable?
const CACHE_MEM_LIMIT: usize = 51200000;

struct CachedPl {
    pl: u32,
    bytes: Vec<u8>,
}

impl Default for CachedPl {
    fn default() -> Self {
        CachedPl { pl: std::u32::MAX, bytes: Vec::new() }
    }
}

/// LRU cache of postings lists, to prevent unnecessary repeated loading from disk / network
pub struct PostingsListCache {
    cache: Vec<CachedPl>,
    cache_size: usize,
}

impl PostingsListCache {
    pub fn new() -> PostingsListCache {
        PostingsListCache {
            cache: Vec::with_capacity(CACHE_SIZE),
            cache_size: 0,
        }
    }

    pub fn add(&mut self, pl: u32, bytes: Vec<u8>) {
        if self.cache.iter().any(|cached_pl| cached_pl.pl == pl) {
            return;
        }

        while (self.cache_size + bytes.len()) > CACHE_MEM_LIMIT
            || self.cache.len() == CACHE_SIZE
        {
            if let Some(first) = self.cache.first() {
                self.cache_size -= first.bytes.len();
                self.cache.remove(0);
            } else {
                break;
            }
        }

        self.cache_size += bytes.len();
        self.cache.push(CachedPl {
            pl,
            bytes,
        });
    }

    pub fn get(&self, pl: u32) -> Option<&Vec<u8>> {
        self.cache.iter().find_map(|cached_pl| {
            if cached_pl.pl == pl {
                Some(&cached_pl.bytes)
            } else {
                None
            }
        })
    }
}
