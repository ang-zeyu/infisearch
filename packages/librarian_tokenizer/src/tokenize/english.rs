use regex::Regex;

lazy_static! {
  static ref PUNCTUATION_FILTER: Regex = Regex::new(r#"[\[\](){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.]"#).unwrap();
  static ref BOUNDARY_FILTER: Regex = Regex::new(r#"(^\W)|(\W$)"#).unwrap();
  static ref SENTENCE_SPLITTER: Regex = Regex::new(r#"[.?!](\s+|$)"#).unwrap();
}

pub fn tokenize (mut text: String) -> Vec<String> {
  text.make_ascii_lowercase();
  SENTENCE_SPLITTER
    .split(&text)
    .flat_map(|sent_slice| sent_slice.split_whitespace()
      .map(|term_slice| BOUNDARY_FILTER.replace_all(&PUNCTUATION_FILTER.replace_all(term_slice, ""), "").into_owned())
      .filter(|term| {
        let term_byte_len = term.as_bytes().len();
        term_byte_len > 0 && term_byte_len <= 120
      })
    )
    .collect()
}