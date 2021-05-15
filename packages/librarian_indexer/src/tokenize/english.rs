use regex::Regex;

lazy_static! {
  static ref PUNCTUATION_FILTER: Regex = Regex::new(r#"[\[\](){}&|'"`<>#:;~_^=\-‑+*/‘’“”，。《》…—‐•?!,.]"#).unwrap();
  static ref BOUNDARY_FILTER: Regex = Regex::new(r#"(^\W)|(\W$)"#).unwrap();
  static ref SENTENCE_SPLITTER: Regex = Regex::new(r#"[.?!](\s+|$)"#).unwrap();
}

pub fn tokenize(text: &str) -> Vec<String> {
  SENTENCE_SPLITTER
    .split(&text.to_ascii_lowercase())
    .flat_map(|sent| sent.split_whitespace()
      .map(|term| BOUNDARY_FILTER.replace_all(&PUNCTUATION_FILTER.replace_all(term, ""), "").into_owned())
      .filter(|term| {
        let term_byte_len = term.as_bytes().len();
        term_byte_len > 0 && term_byte_len <= 120
      })
    )
    .collect()
}