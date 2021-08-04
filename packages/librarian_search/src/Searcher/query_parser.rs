use librarian_common::tokenize::Tokenizer;
use serde::{Serialize};

#[derive(Serialize)]
pub enum QueryPartType {
  TERM,
  PHRASE,
  BRACKET,
  AND,
  NOT,
  ADDED,
}

#[derive(Serialize)]
pub struct QueryPart {
  #[serde(rename = "isCorrected")]
  pub is_corrected: bool,
  #[serde(rename = "isStopWordRemoved")]
  pub is_stop_word_removed: bool,
  #[serde(rename = "shouldExpand")]
  pub should_expand: bool,
  #[serde(rename = "isExpanded")]
  pub is_expanded: bool,
  #[serde(rename = "originalTerms")]
  pub original_terms: Option<Vec<String>>,
  pub terms: Option<Vec<String>>,
  #[serde(rename = "partType")]
  pub part_type: QueryPartType,
  #[serde(rename = "fieldName")]
  pub field_name: Option<String>,
  pub children: Option<Vec<QueryPart>>,
}

enum QueryParseState {
    NONE,
    QUOTE,
    PARENTHESES,
}

pub fn parse_query(query: String, tokenizer: &Box<dyn Tokenizer>) -> Result<Vec<QueryPart>, &'static str> {
  let mut query_parts: Vec<QueryPart> = Vec::with_capacity(5);

  let mut query_parse_state: QueryParseState = QueryParseState::NONE;
  let mut is_expecting_and = false;
  let mut is_not_allowed = true;
  let mut did_encounter_not = false;
  let mut field_name: Option<String> = None;

  let mut i = 0;
  let mut j = 0;
  let mut last_whitespace_idx = 0;

  let wrap_in_not = |mut query_part: QueryPart, did_encounter_not: &mut bool, field_name: &mut Option<String>| -> QueryPart {
    if *did_encounter_not {
      *did_encounter_not = false;
      QueryPart {
        is_corrected: false,
        is_stop_word_removed: false,
        should_expand: false,
        is_expanded: false,
        original_terms: Option::None,
        terms: Option:: None,
        part_type: QueryPartType::NOT,
        field_name: std::mem::take(field_name),
        children: Option::from(vec![query_part]),
      }
    } else {
      query_part.field_name = std::mem::take(field_name);
      query_part
    }
  };

  let handle_free_text = |query_parts: &mut Vec<QueryPart>, chars: &Vec<char>, i: usize, j: usize, did_encounter_not: &mut bool, field_name: &mut Option<String>| {
    if i == j {
      return;
    }

    let tokenize_result = tokenizer.wasm_tokenize(chars[i..j].iter().collect());
    if tokenize_result.terms.len() == 0 {
      return;
    }

    let mut is_first = true;
    for term in tokenize_result.terms {
      if is_first {
        is_first = false;

        query_parts.push(wrap_in_not(QueryPart {
          is_corrected: false,
          is_stop_word_removed: false,
          should_expand: tokenize_result.should_expand,
          is_expanded: false,
          original_terms: Option::None,
          terms: Option::from(vec![term]),
          part_type: QueryPartType::TERM,
          field_name: None,
          children: Option::None,
        }, did_encounter_not, field_name));
      } else {
        query_parts.push(QueryPart {
          is_corrected: false,
          is_stop_word_removed: false,
          should_expand: tokenize_result.should_expand,
          is_expanded: false,
          original_terms: Option::None,
          terms: Option::from(vec![term]),
          part_type: QueryPartType::TERM,
          field_name: None,
          children: Option::None,
        });
      }
    }
  };

  let query_chars: Vec<char> = query.chars().collect();
  let query_chars_len = query_chars.len();
  while j < query_chars_len {
    let c = query_chars[j];

    match query_parse_state {
      QueryParseState::QUOTE => {
        if c == '"' {
          query_parse_state = QueryParseState::NONE;
          
          let terms = tokenizer.wasm_tokenize(query_chars[i..j].iter().collect()).terms;
          let part_type = if terms.len() <= 1 { QueryPartType::TERM } else { QueryPartType::PHRASE };
          let phrase_query_part: QueryPart = wrap_in_not(QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: false,
            is_expanded: false,
            original_terms: Option::None,
            terms: Option::from(terms),
            part_type,
            field_name: None,
            children: Option::None,
          }, &mut did_encounter_not, &mut field_name);

          if is_expecting_and {
            let last_query_part_idx = query_parts.len() - 1;
            query_parts.get_mut(last_query_part_idx).unwrap()
              .children.as_mut().unwrap()
              .push(phrase_query_part);
            is_expecting_and = false;
          } else {
            query_parts.push(phrase_query_part);
          }

          i = j + 1;

          is_not_allowed = true;
        }
      },
      QueryParseState::PARENTHESES => {
        if c == ')' {
          query_parse_state = QueryParseState::NONE;
          
          let content: String = query_chars[i..j].iter().collect();
          let child_query_part: QueryPart = wrap_in_not(QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: false,
            is_expanded: false,
            original_terms: Option::None,
            terms: Option::None,
            part_type: QueryPartType::BRACKET,
            field_name: None,
            children: Option::from(parse_query(content, tokenizer)?),
          }, &mut did_encounter_not, &mut field_name);

          if is_expecting_and {
            let last_query_part_idx = query_parts.len() - 1;
            query_parts.get_mut(last_query_part_idx).unwrap()
              .children.as_mut().unwrap()
              .push(child_query_part);
            is_expecting_and = false;
          } else {
            query_parts.push(child_query_part);
          }

          i = j + 1;

          is_not_allowed = true;
        }
      },
      QueryParseState::NONE => {
        if c == '"' || c == '(' {
          if is_expecting_and {
            if i != j {
              let mut curr_query_parts = parse_query(query_chars[i..j].iter().collect(), tokenizer)?;

              let last_query_part_idx = query_parts.len() - 1;
              query_parts.get_mut(last_query_part_idx).unwrap()
                .children.as_mut().unwrap()
                .push(wrap_in_not(curr_query_parts.remove(0), &mut did_encounter_not, &mut field_name));
              query_parts.append(&mut curr_query_parts);
              is_expecting_and = false;
            }
            // i === j: the phrase / parentheses is part of the AND (e.g. lorem AND (ipsum))
          } else {
            handle_free_text(&mut query_parts, &query_chars, i, j, &mut did_encounter_not, &mut field_name);
          }

          query_parse_state = if c == '"' { QueryParseState::QUOTE } else { QueryParseState::PARENTHESES };
          i = j + 1;
        } else if c == ':' && last_whitespace_idx >= i && j > i {
          if is_expecting_and {
            if i != last_whitespace_idx {
              let mut curr_query_parts = parse_query(query_chars[i..last_whitespace_idx].iter().collect(), tokenizer)?;
  
              let last_query_part_idx = query_parts.len() - 1;
              query_parts.get_mut(last_query_part_idx).unwrap()
                .children.as_mut().unwrap()
                .push(wrap_in_not(curr_query_parts.remove(0), &mut did_encounter_not, &mut field_name));
              query_parts.append(&mut curr_query_parts);
              is_expecting_and = false;
            }
          } else {
            handle_free_text(&mut query_parts, &query_chars, i, last_whitespace_idx, &mut did_encounter_not, &mut field_name);
          }
          
          field_name = Some(query_chars[last_whitespace_idx..j].iter().collect());

          i = j + 1;
        } else if c.is_ascii_whitespace() {
          let initial_j = j;
          while j < query_chars_len && query_chars[j].is_ascii_whitespace() {
            j += 1;
          }

          if query_chars_len > 6 // overflow
            &&  j < query_chars_len - 4
            && query_chars[j] == 'A' && query_chars[j + 1] == 'N' && query_chars[j + 2] == 'D'
            && query_chars[j + 3].is_ascii_whitespace() {
            
            let mut curr_query_parts = parse_query(query_chars[i..initial_j].iter().collect(), tokenizer)?;

            if curr_query_parts.len() > 0 {
              let first_query_part = wrap_in_not(curr_query_parts.swap_remove(0), &mut did_encounter_not, &mut field_name);
              curr_query_parts.push(first_query_part);
              let last_query_part = curr_query_parts.swap_remove(0);
              curr_query_parts.push(last_query_part);

              if is_expecting_and {
                let last_query_part_idx = query_parts.len() - 1;
                query_parts.get_mut(last_query_part_idx).unwrap()
                  .children.as_mut().unwrap()
                  .push(curr_query_parts.remove(0));
              }

              if curr_query_parts.len() > 0 {
                // A new, disjoint AND group from the previous (if any)
                let last_curr_query_part = curr_query_parts.pop().unwrap();
                query_parts.append(&mut curr_query_parts);
                query_parts.push(QueryPart {
                  is_corrected: false,
                  is_stop_word_removed: false,
                  should_expand: false,
                  is_expanded: false,
                  original_terms: Option::None,
                  terms: Option::None,
                  part_type: QueryPartType::AND,
                  field_name: std::mem::take(&mut field_name),
                  children: Option::from(vec![last_curr_query_part]),
                });
              }
            } else if query_parts.len() > 0 && !is_expecting_and {
              // e.g. (lorem) AND ipsum
              let last_curr_query_part = query_parts.pop().unwrap();
              if let QueryPartType::AND = last_curr_query_part.part_type {
                // Reuse last AND group
                query_parts.push(last_curr_query_part);
              } else {
                query_parts.push(QueryPart {
                  is_corrected: false,
                  is_stop_word_removed: false,
                  should_expand: false,
                  is_expanded: false,
                  original_terms: Option::None,
                  terms: Option::None,
                  part_type: QueryPartType::AND,
                  field_name: std::mem::take(&mut field_name),
                  children: Option::from(vec![last_curr_query_part]),
                });
              }
            } else {
              return Err("Query parsing error: no token found before AND operator");
            }
            is_expecting_and = true;

            j += 4;
            while j < query_chars_len && query_chars[j].is_ascii_whitespace() {
              j += 1;
            }
            i = j;
          }

          last_whitespace_idx = j;
          j -= 1;
          is_not_allowed = true;
        } else if is_not_allowed
          && query_chars_len > 5 // overflow
          && j < query_chars_len - 4
          && query_chars[j] == 'N' && query_chars[j + 1] == 'O' && query_chars[j + 2] == 'T'
          && query_chars[j + 3].is_ascii_whitespace() {
          let mut curr_query_parts = parse_query(query_chars[i..j].iter().collect(), tokenizer)?;
          if curr_query_parts.len() > 0 {
            let first_query_part = wrap_in_not(curr_query_parts.swap_remove(0), &mut did_encounter_not, &mut field_name);
            curr_query_parts.push(first_query_part);
            let last_query_part = curr_query_parts.swap_remove(0);
            curr_query_parts.push(last_query_part);

            if is_expecting_and {
              let last_query_part_idx = query_parts.len() - 1;
              query_parts.get_mut(last_query_part_idx).unwrap()
                .children.as_mut().unwrap()
                .push(curr_query_parts.remove(0));
              is_expecting_and = false;
            }

            query_parts.append(&mut curr_query_parts);
          }
          did_encounter_not = true;

          j += 4;
          while j < query_chars_len && query_chars[j].is_ascii_whitespace() {
            j += 1;
          }
          i = j;
          j -= 1;
        } else {
          is_not_allowed = false;
        }
      }
    }

    j += 1;
  }

  if is_expecting_and {
    if i != j {
      let mut curr_query_parts = parse_query(query_chars[i..j].iter().collect(), tokenizer)?;

      let last_query_part_idx = query_parts.len() - 1;
      query_parts.get_mut(last_query_part_idx).unwrap()
        .children.as_mut().unwrap()
        .push(wrap_in_not(curr_query_parts.remove(0), &mut did_encounter_not, &mut field_name));
      query_parts.append(&mut curr_query_parts);
    } else {
      return Err("Query parsing error: no token found after AND operator");
    }
  } else {
    handle_free_text(&mut query_parts, &query_chars, i, j, &mut did_encounter_not, &mut field_name);
  }

  Ok(query_parts)
}
