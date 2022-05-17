#[inline(always)]
pub fn get_tri_grams(term: &str) -> impl Iterator<Item = &str> {
    let chars = term.char_indices();

    chars
        .enumerate()
        .scan((0, 0), move |acc, idx_and_charindex| {
            let char_idx = idx_and_charindex.1.0;
            let ret = Some(&term[acc.0..char_idx + idx_and_charindex.1.1.len_utf8()]);
            if idx_and_charindex.0 >= 2 {
                acc.0 = acc.1;
                acc.1 = char_idx;
            } else if idx_and_charindex.0 == 1 {
                acc.1 = char_idx;
            }
            ret
        })
}

#[cfg(test)]
mod test {
    use super::get_tri_grams;

    #[test]
    fn test_trigram_extraction() {
        assert!(get_tri_grams("").next().is_none());
        assert!(get_tri_grams("f").eq(vec!["f"]));
        assert!(get_tri_grams("fo").eq(vec!["f", "fo"]));
        assert!(get_tri_grams("foo").eq(vec!["f", "fo", "foo"]));
        assert!(get_tri_grams("foobar").eq(vec!["f", "fo", "foo", "oob", "oba", "bar"]));
    }
}
