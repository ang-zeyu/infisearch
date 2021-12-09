use std::borrow::Cow;

use regex::Regex;

lazy_static! {
    static ref TERM_FILTER: Regex =
        Regex::new(r#"(^\W+)|(\W+$)|([\[\]\\(){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.])"#).unwrap();
}

// Custom replace_all regex implementation accepting cow to make lifetimes comply
// https://github.com/rust-lang/regex/issues/676
#[inline(always)]
pub fn term_filter(input: Cow<str>) -> Cow<str> {
    let mut match_iter = TERM_FILTER.find_iter(&input);
    if let Some(first) = match_iter.next() {
        let mut output: Vec<u8> = Vec::with_capacity(input.len());
        output.extend_from_slice(input[..first.start()].as_bytes());
        let mut start = first.end();

        loop {
            if let Some(next) = match_iter.next() {
                output.extend_from_slice(input[start..next.start()].as_bytes());
                start = next.end();
            } else {
                output.extend_from_slice(input[start..].as_bytes());
                return Cow::Owned(unsafe { String::from_utf8_unchecked(output) });
            }
        }
    } else {
        input
    }
}
