use crate::tokenize::WasmTokenizeResult;
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

pub fn wasm_tokenize(text: String) -> WasmTokenizeResult {
  let should_expand = !text.ends_with(" ");
  return WasmTokenizeResult {
    terms: tokenize(text),
    should_expand,
  }
}

pub fn get_stop_words() -> &'static str {
  // from tantivy
  r#"[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into",
    "is", "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then",
    "there", "these", "they", "this", "to", "was", "will", "with"
  ]"#
}
