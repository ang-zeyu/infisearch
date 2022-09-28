/// Split iterator that keeps all separators as separate slices
/// This allows splitting on chinese characters easily,
/// tokenizing other slices as ascii characters.
pub struct SplitIncl<'a, F> where F: Fn(char) -> bool {
    s: &'a str,
    idx: usize,
    char_idx: usize,
    is_delimiter: F,
}

impl<'a, F> SplitIncl<'a, F> where F: Fn(char) -> bool {
    pub fn split(s: &'a str, is_delimiter: F) -> SplitIncl<'a, F> {
        SplitIncl { s, idx: 0, char_idx: 0, is_delimiter }
    }
}

impl<'a, F> Iterator for SplitIncl<'a, F> where F: Fn(char) -> bool {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<(usize, &'a str)> {
        if self.idx >= self.s.len() {
            return None;
        }

        let remaining_slice = &self.s[self.idx..];
        let mut seen_first = false;
        for (char_idx, (idx, c)) in remaining_slice.char_indices().chain(std::iter::once((remaining_slice.len(), ','))).enumerate() {
            if (self.is_delimiter)(c) {
                if seen_first {
                    let ret = (self.char_idx, &remaining_slice[..idx]);
                    self.idx += idx;
                    self.char_idx += char_idx;
                    return Some(ret);
                } else {
                    debug_assert!(idx == 0);
                    let len = c.len_utf8();
                    self.idx += len;
                    self.char_idx += 1;
                    return Some((1, &remaining_slice[..len]));
                }
            }

            seen_first = true;
        }

        None
    }
}
