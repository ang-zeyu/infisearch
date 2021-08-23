use morsels_common::tokenize::Tokenizer;
use serde::{Serialize};

#[derive(Serialize, Debug, Eq, PartialEq)]
pub enum QueryPartType {
  TERM,
  PHRASE,
  BRACKET,
  AND,
  NOT,
  ADDED,
}

#[derive(Serialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueryPart {
  pub is_corrected: bool,
  pub is_stop_word_removed: bool,
  pub should_expand: bool,
  pub is_expanded: bool,
  pub original_terms: Option<Vec<String>>,
  pub terms: Option<Vec<String>>,
  pub part_type: QueryPartType,
  pub field_name: Option<String>,
  pub children: Option<Vec<QueryPart>>,
}

enum UnaryOp {
  NOT,
  FIELD(String)
}

enum QueryParseState {
    NONE,
    QUOTE,
    PARENTHESES,
}

fn handle_unary_op(mut query_part: QueryPart, operator_stack: &mut Vec<UnaryOp>) -> QueryPart {
  while !operator_stack.is_empty() {
    let op = operator_stack.pop().unwrap();
    match op {
      UnaryOp::NOT => {
        query_part = QueryPart {
          is_corrected: false,
          is_stop_word_removed: false,
          should_expand: false,
          is_expanded: false,
          original_terms: None,
          terms: None,
          part_type: QueryPartType::NOT,
          field_name: None,
          children: Some(vec![query_part]),
        }
      },
      UnaryOp::FIELD(field_name) => {
        query_part.field_name = Some(field_name);
      },
    }
  }
  query_part
}

fn collect_slice(query_chars: &Vec<char>, i: usize, j: usize, escape_indices: &Vec<usize>) -> String {
  query_chars[i..j].iter().enumerate()
    .filter(|(idx, _char)| escape_indices.iter().find(|escape_idx| **escape_idx == (*idx + i)).is_none())
    .map(|(_idx, c)| c)
    .collect()
}

fn handle_terminator(
  tokenizer: &Box<dyn Tokenizer>,
  query_chars: &Vec<char>,
  i: usize, j: usize,
  escape_indices: &Vec<usize>,
  query_parts: &mut Vec<QueryPart>,
  is_expecting_and: &mut bool,
  operator_stack: &mut Vec<UnaryOp>,
) {
  if i == j {
    return;
  }

  if *is_expecting_and {
    if i != j {
      let mut curr_query_parts = parse_query(collect_slice(query_chars, i, j, escape_indices), tokenizer);

      if curr_query_parts.len() > 0 {
        let last_query_part_idx = query_parts.len() - 1;
        query_parts.get_mut(last_query_part_idx).unwrap()
          .children.as_mut().unwrap()
          .push(handle_unary_op(curr_query_parts.remove(0), operator_stack));
        query_parts.append(&mut curr_query_parts);
      }

      *is_expecting_and = false;
    }
  } else {
    let tokenize_result = tokenizer.wasm_tokenize(collect_slice(query_chars, i, j, &escape_indices));
    if tokenize_result.terms.len() == 0 {
      return;
    }
  
    let mut is_first = true;
    for term in tokenize_result.terms {
      if is_first {
        is_first = false;
  
        query_parts.push(handle_unary_op(QueryPart {
          is_corrected: false,
          is_stop_word_removed: false,
          should_expand: tokenize_result.should_expand,
          is_expanded: false,
          original_terms: None,
          terms: Some(vec![term]),
          part_type: QueryPartType::TERM,
          field_name: None,
          children: None,
        }, operator_stack));
      } else {
        query_parts.push(QueryPart {
          is_corrected: false,
          is_stop_word_removed: false,
          should_expand: tokenize_result.should_expand,
          is_expanded: false,
          original_terms: None,
          terms: Some(vec![term]),
          part_type: QueryPartType::TERM,
          field_name: None,
          children: None,
        });
      }
    }
  }
}

pub fn parse_query(query: String, tokenizer: &Box<dyn Tokenizer>) -> Vec<QueryPart> {
  let mut query_parts: Vec<QueryPart> = Vec::with_capacity(5);

  let mut query_parse_state: QueryParseState = QueryParseState::NONE;
  let mut is_expecting_and = false;
  let mut did_encounter_escape = false;
  let mut escape_indices: Vec<usize> = Vec::new();
  let mut op_stack: Vec<UnaryOp> = Vec::new();

  let mut i = 0;
  let mut j = 0;
  let mut last_possible_unaryop_idx = 0;

  let query_chars: Vec<char> = query.chars().collect();
  let query_chars_len = query_chars.len();

  while j < query_chars_len {
    let c = query_chars[j];

    match query_parse_state {
      QueryParseState::QUOTE | QueryParseState::PARENTHESES => {
        let char_to_match = if let QueryParseState::QUOTE = query_parse_state { '"' } else { ')' };
        if !did_encounter_escape && c == char_to_match {
          let content = collect_slice(&query_chars, i, j, &escape_indices);
          let term_parttype_children = if let QueryParseState::QUOTE = query_parse_state {
            (Some(tokenizer.wasm_tokenize(content).terms), QueryPartType::PHRASE, None)
          } else {
            (None, QueryPartType::BRACKET, Some(parse_query(content, tokenizer)))
          };
          query_parse_state = QueryParseState::NONE;

          let query_part: QueryPart = handle_unary_op(QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: false,
            is_expanded: false,
            original_terms: None,
            terms: term_parttype_children.0,
            part_type: term_parttype_children.1,
            field_name: None,
            children: term_parttype_children.2
          }, &mut op_stack);

          if is_expecting_and {
            let last_query_part_idx = query_parts.len() - 1;
            query_parts.get_mut(last_query_part_idx).unwrap()
              .children.as_mut().unwrap()
              .push(query_part);
            is_expecting_and = false;
          } else {
            query_parts.push(query_part);
          }

          i = j + 1;
          last_possible_unaryop_idx = i;
        } else if c == '\\' {
          did_encounter_escape = true;
        } else {
          did_encounter_escape = false;
        }
      }
      QueryParseState::NONE => {
        if !did_encounter_escape && (c == '"' || c == '(') {
          handle_terminator(
            tokenizer, &query_chars,
            i, j, &escape_indices,
            &mut query_parts, &mut is_expecting_and, &mut op_stack
          );

          query_parse_state = if c == '"' { QueryParseState::QUOTE } else { QueryParseState::PARENTHESES };
          i = j + 1;
        } else if c == ':' && !did_encounter_escape && last_possible_unaryop_idx >= i && j > i {
          handle_terminator(
            tokenizer, &query_chars,
            i, last_possible_unaryop_idx,
            &escape_indices, &mut query_parts, &mut is_expecting_and, &mut op_stack
          );
          
          op_stack.push(UnaryOp::FIELD(collect_slice(&query_chars, last_possible_unaryop_idx, j, &escape_indices)));
          i = j + 1;
          last_possible_unaryop_idx = i;
        } else if c.is_ascii_whitespace() {
          let initial_j = j;
          while j < query_chars_len && query_chars[j].is_ascii_whitespace() {
            j += 1;
          }

          if !did_encounter_escape
            && query_chars_len > 6 // overflow
            &&  j < query_chars_len - 4
            && query_chars[j] == 'A' && query_chars[j + 1] == 'N' && query_chars[j + 2] == 'D'
            && query_chars[j + 3].is_ascii_whitespace() {
            handle_terminator(
              tokenizer, &query_chars,
              i, initial_j, &escape_indices,
              &mut query_parts, &mut is_expecting_and, &mut op_stack
            );
              
            let last_curr_query_part = query_parts.pop();
            if last_curr_query_part.is_some()
              && matches!(last_curr_query_part.as_ref().unwrap().part_type, QueryPartType::AND) {
              // Reuse last AND group
              query_parts.push(last_curr_query_part.unwrap());
            } else {
              query_parts.push(QueryPart {
                is_corrected: false,
                is_stop_word_removed: false,
                should_expand: false,
                is_expanded: false,
                original_terms: None,
                terms: None,
                part_type: QueryPartType::AND,
                field_name: None,
                children: Some(if let Some(last_curr_query_part) = last_curr_query_part {
                  vec![last_curr_query_part]
                } else {
                  vec![]
                }),
              });
            }
            is_expecting_and = true;

            j += 4;
            while j < query_chars_len && query_chars[j].is_ascii_whitespace() {
              j += 1;
            }
            i = j;
          }

          last_possible_unaryop_idx = j;
          j -= 1;
        } else if j == last_possible_unaryop_idx
          && !did_encounter_escape
          && query_chars_len > 5 // overflow
          && j < query_chars_len - 4
          && query_chars[j] == 'N' && query_chars[j + 1] == 'O' && query_chars[j + 2] == 'T'
          && query_chars[j + 3].is_ascii_whitespace() {
          handle_terminator(
            tokenizer, &query_chars,
            i, j, &escape_indices,
            &mut query_parts, &mut is_expecting_and, &mut op_stack
          );
          
          op_stack.push(UnaryOp::NOT);

          j += 4;
          while j < query_chars_len && query_chars[j].is_ascii_whitespace() {
            j += 1;
          }
          i = j;
          last_possible_unaryop_idx = i;
          j -= 1;
        } else if c == '\\' {
          did_encounter_escape = !did_encounter_escape;
          if did_encounter_escape {
            escape_indices.push(j);
          }
        } else {
          did_encounter_escape = false;
        }
      }
    }

    j += 1;
  }

  handle_terminator(tokenizer, &query_chars, i, j, &escape_indices, &mut query_parts, &mut is_expecting_and, &mut op_stack);

  query_parts
}

#[cfg(test)]
mod test {
  use pretty_assertions::{assert_eq};

  use morsels_common::tokenize::Tokenizer;
  use morsels_lang_latin::english::{self, EnglishTokenizerOptions};

  use super::{QueryPart, QueryPartType};

  impl QueryPart {
    fn no_expand(mut self) -> QueryPart {
      if let QueryPartType::TERM = self.part_type {
        self.should_expand = false;
        self
      } else {
        panic!("Tried to call toggle_should_expand test function on non-term query part");
      }
    }

    fn with_field(mut self, field_name: &str) -> QueryPart {
      self.field_name = Some(field_name.to_owned());
      self
    }
  }

  fn wrap_in_not(query_part: QueryPart) -> QueryPart {
    QueryPart {
      is_corrected: false,
      is_stop_word_removed: false,
      should_expand: false,
      is_expanded: false,
      original_terms: None,
      terms: None,
      part_type: QueryPartType::NOT,
      field_name: None,
      children: Some(vec![query_part]),
    }
  }

  fn wrap_in_and(query_parts: Vec<QueryPart>) -> QueryPart {
    QueryPart {
      is_corrected: false,
      is_stop_word_removed: false,
      should_expand: false,
      is_expanded: false,
      original_terms: None,
      terms: None,
      part_type: QueryPartType::AND,
      field_name: None,
      children: Some(query_parts),
    }
  }

  fn wrap_in_parentheses(query_parts: Vec<QueryPart>) -> QueryPart {
    QueryPart {
      is_corrected: false,
      is_stop_word_removed: false,
      should_expand: false,
      is_expanded: false,
      original_terms: None,
      terms: None,
      part_type: QueryPartType::BRACKET,
      field_name: None,
      children: Some(query_parts),
    }
  }

  fn get_term(term: &str) -> QueryPart {
    QueryPart {
      is_corrected: false,
      is_stop_word_removed: false,
      should_expand: true,
      is_expanded: false,
      original_terms: None,
      terms: Some(vec![term.to_owned()]),
      part_type: QueryPartType::TERM,
      field_name: None,
      children: None,
    }
  }

  fn get_lorem() -> QueryPart {
    get_term("lorem")
  }

  fn get_ipsum() -> QueryPart {
    get_term("ipsum")
  }

  fn parse(query: &str) -> Vec<QueryPart> {
    let tokenizer: Box<dyn Tokenizer> = Box::new(english::new_with_options(EnglishTokenizerOptions {
      stop_words: None,
      stemmer: None,
      max_term_len: 80,
    }));

    super::parse_query(query.to_owned(), &tokenizer)
  }

  #[test]
  fn free_text_test() {
    assert_eq!(parse("lorem ipsum"), vec![get_lorem(), get_ipsum()]);
    assert_eq!(parse("lorem ipsum "), vec![get_lorem().no_expand(), get_ipsum().no_expand()]);
  }

  #[test]
  fn boolean_test() {
    assert_eq!(parse("NOT lorem"), vec![wrap_in_not(get_lorem())]);
    assert_eq!(parse("NOT NOT lorem"), vec![wrap_in_not(wrap_in_not(get_lorem()))]);
    assert_eq!(parse("NOT lorem ipsum"), vec![wrap_in_not(get_lorem()), get_ipsum()]);
    assert_eq!(parse("lorem NOTipsum"), vec![get_lorem(), get_term("notipsum")]);
    assert_eq!(parse("lorem NOT ipsum"), vec![get_lorem().no_expand(), wrap_in_not(get_ipsum())]);
    assert_eq!(parse("lorem AND ipsum"), vec![wrap_in_and(vec![get_lorem(), get_ipsum()])]);
    assert_eq!(parse("lorem AND ipsum AND lorem"), vec![wrap_in_and(vec![get_lorem(), get_ipsum(), get_lorem()])]);
    assert_eq!(parse("lorem AND NOT ipsum"), vec![wrap_in_and(vec![get_lorem(), wrap_in_not(get_ipsum())])]);
    assert_eq!(parse("NOT lorem AND NOT ipsum"), vec![wrap_in_and(vec![wrap_in_not(get_lorem()), wrap_in_not(get_ipsum())])]);
    assert_eq!(parse("NOT lorem AND NOT ipsum lorem NOT ipsum"), vec![
      wrap_in_and(vec![wrap_in_not(get_lorem()), wrap_in_not(get_ipsum().no_expand())]),
      get_lorem().no_expand(),
      wrap_in_not(get_ipsum())
    ]);
  }

  #[test]
  fn parentheses_test() {
    assert_eq!(parse("(lorem ipsum)"), vec![wrap_in_parentheses(vec![get_lorem(), get_ipsum()])]);
    assert_eq!(parse("(lorem ipsum )"), vec![wrap_in_parentheses(vec![get_lorem().no_expand(), get_ipsum().no_expand()])]);
    assert_eq!(parse("lorem(lorem ipsum)"), vec![
      get_lorem(),
      wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
    ]);
    assert_eq!(parse("(lorem ipsum)lorem(lorem ipsum)"), vec![
      wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
      get_lorem(),
      wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
    ]);
    assert_eq!(parse("(lorem ipsum) lorem (lorem ipsum)"), vec![
      wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
      get_lorem().no_expand(),
      wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
    ]);
  }

  #[test]
  fn field_name_test() {
    assert_eq!(parse("title:lorem"), vec![get_lorem().with_field("title")]);
    assert_eq!(parse("title:lorem ipsum"), vec![get_lorem().with_field("title"), get_ipsum()]);
    assert_eq!(parse("title:lorem body:ipsum"), vec![get_lorem().with_field("title").no_expand(), get_ipsum().with_field("body")]);
    assert_eq!(parse("title:(lorem body:ipsum)"), vec![
      wrap_in_parentheses(
        vec![get_lorem().no_expand(), get_ipsum().with_field("body")]
      ).with_field("title")
    ]);
    assert_eq!(parse("title:lorem AND ipsum"), vec![wrap_in_and(vec![get_lorem().with_field("title"), get_ipsum()])]);
    assert_eq!(parse("title:(lorem AND ipsum)"), vec![wrap_in_parentheses(vec![
      wrap_in_and(vec![get_lorem(), get_ipsum()])]
    ).with_field("title")]);
    assert_eq!(parse("title:NOT lorem ipsum)"), vec![wrap_in_not(get_lorem()).with_field("title"), get_ipsum()]);
    assert_eq!(parse("title: NOT lorem ipsum)"), vec![wrap_in_not(get_lorem()).with_field("title"), get_ipsum()]);
    assert_eq!(parse("title: lorem NOT ipsum)"), vec![get_lorem().with_field("title").no_expand(), wrap_in_not(get_ipsum())]);
  }

  #[test]
  fn misc_test() {
    assert_eq!(parse("title:(lorem AND ipsum) AND NOT (lorem ipsum) body:(lorem NOT ipsum)"), vec![
      wrap_in_and(vec![
        wrap_in_parentheses(vec![
          wrap_in_and(vec![
            get_lorem(), get_ipsum()
          ])
        ]).with_field("title"),
        wrap_in_not(wrap_in_parentheses(vec![
          get_lorem(), get_ipsum(),
        ]))
      ]),
      wrap_in_parentheses(vec![
        get_lorem().no_expand(),
        wrap_in_not(get_ipsum())
      ]).with_field("body")
    ]);

    assert_eq!(parse("title:lorem AND ipsum AND NOT lorem ipsum body:lorem NOT ipsum"), vec![
      wrap_in_and(vec![
        get_lorem().with_field("title"),
        get_ipsum(),
        wrap_in_not(get_lorem().no_expand()),
      ]),
      get_ipsum().no_expand(),
      get_lorem().no_expand().with_field("body"),
      wrap_in_not(get_ipsum()),
    ]);

    assert_eq!(parse("title\\:lorem\\ AND ipsum\\ AND \\NOT lorem ipsum body\\:lorem \\NOT ipsum"), vec![
      get_term("titlelorem"),
      get_term("and"),
      get_term("ipsum"),
      get_term("and"),
      get_term("not"),
      get_term("lorem"),
      get_term("ipsum"),
      get_term("bodylorem"),
      get_term("not"),
      get_term("ipsum"),
    ]);
  }
}
