use std::borrow::Cow;

use regex::Regex;

// Adapted from https://lise-henry.github.io/articles/optimising_strings.html
pub fn escape<'a, S: Into<Cow<'a, str>>>(input: S) -> Cow<'a, str> {
    lazy_static! {
        static ref REGEX: Regex = Regex::new(r#"[\n\r\t"\\\x08\x0c]"#).unwrap();
    }
    let input = input.into();
    let first = REGEX.find(&input);
    if let Some(first) = first {
        let start = first.start();
        let len = input.len();
        let mut output: Vec<u8> = Vec::with_capacity(len + len / 2);
        output.extend_from_slice(input[0..start].as_bytes());
        let rest = input[start..].bytes();
        for c in rest {
            match c {
                8 => output.extend_from_slice(b"\\b"),
                12 => output.extend_from_slice(b"\\f"),
                b'\n' => output.extend_from_slice(b"\\n"),
                b'\r' => output.extend_from_slice(b"\\r"),
                b'\t' => output.extend_from_slice(b"\\t"),
                b'"' => output.extend_from_slice(b"\\\""),
                b'\\' => output.extend_from_slice(b"\\\\"),
                // All other control characters should use unicode escape sequences
                0..=31 => output.extend_from_slice(format!("\\u00{:02X?}", c).as_bytes()),
                _ => output.push(c),
            }
        }
        Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
    } else {
        input
    }
}
