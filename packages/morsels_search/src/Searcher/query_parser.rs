use morsels_common::tokenize::Tokenizer;
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

fn wrap_in_not(mut query_part: QueryPart, did_encounter_not: &mut bool, field_name: &mut Option<String>) -> QueryPart {
  query_part.field_name = std::mem::take(field_name);

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
      field_name: None,
      children: Option::from(vec![query_part]),
    }
  } else {
    query_part
  }
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
  did_encounter_not: &mut bool,
  field_name: &mut Option<String>,
) -> Result<(), &'static str> {
  if i == j {
    return Ok(());
  }

  if *is_expecting_and {
    if i != j {
      let mut curr_query_parts = parse_query(collect_slice(query_chars, i, j, escape_indices), tokenizer)?;

      if curr_query_parts.len() > 0 {
        let last_query_part_idx = query_parts.len() - 1;
        query_parts.get_mut(last_query_part_idx).unwrap()
          .children.as_mut().unwrap()
          .push(wrap_in_not(curr_query_parts.remove(0), did_encounter_not, field_name));
        query_parts.append(&mut curr_query_parts);
      }

      *is_expecting_and = false;
    }
  } else {
    let tokenize_result = tokenizer.wasm_tokenize(collect_slice(query_chars, i, j, &escape_indices));
    if tokenize_result.terms.len() == 0 {
      return Ok(());
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
  }

  Ok(())
}

pub fn parse_query(query: String, tokenizer: &Box<dyn Tokenizer>) -> Result<Vec<QueryPart>, &'static str> {
  let mut query_parts: Vec<QueryPart> = Vec::with_capacity(5);

  let mut query_parse_state: QueryParseState = QueryParseState::NONE;
  let mut is_expecting_and = false;
  let mut is_not_allowed = true;
  let mut did_encounter_not = false;
  let mut did_encounter_escape = false;
  let mut escape_indices: Vec<usize> = Vec::new();
  let mut field_name: Option<String> = None;

  let mut i = 0;
  let mut j = 0;
  let mut last_possible_fieldname_idx = 0;

  let query_chars: Vec<char> = query.chars().collect();
  let query_chars_len = query_chars.len();

  while j < query_chars_len {
    let c = query_chars[j];

    match query_parse_state {
      QueryParseState::QUOTE | QueryParseState::PARENTHESES => {
        let char_to_match = if let QueryParseState::QUOTE = query_parse_state { '"' } else { ')' };
        if !did_encounter_escape && c == char_to_match {
          let content: String = collect_slice(&query_chars, i, j, &escape_indices);
          let term_parttype_children = if let QueryParseState::QUOTE = query_parse_state {
            (Some(tokenizer.wasm_tokenize(content).terms), QueryPartType::PHRASE, None)
          } else {
            (None, QueryPartType::BRACKET, Some(parse_query(content, tokenizer)?))
          };
          query_parse_state = QueryParseState::NONE;

          let query_part: QueryPart = wrap_in_not(QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: false,
            is_expanded: false,
            original_terms: None,
            terms: term_parttype_children.0,
            part_type: term_parttype_children.1,
            field_name: None,
            children: term_parttype_children.2
          }, &mut did_encounter_not, &mut field_name);

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
          last_possible_fieldname_idx = i;

          is_not_allowed = true;
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
            &mut query_parts, &mut is_expecting_and, &mut did_encounter_not, &mut field_name
          )?;

          query_parse_state = if c == '"' { QueryParseState::QUOTE } else { QueryParseState::PARENTHESES };
          i = j + 1;
        } else if c == ':' && !did_encounter_escape && last_possible_fieldname_idx >= i && j > i {
          handle_terminator(
            tokenizer, &query_chars,
            i, last_possible_fieldname_idx,
            &escape_indices, &mut query_parts, &mut is_expecting_and, &mut did_encounter_not, &mut field_name
          )?;
          
          field_name = Some(collect_slice(&query_chars, last_possible_fieldname_idx, j, &escape_indices));
          i = j + 1;
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
              &mut query_parts, &mut is_expecting_and, &mut did_encounter_not, &mut field_name
            )?;
              
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
                original_terms: Option::None,
                terms: Option::None,
                part_type: QueryPartType::AND,
                field_name: std::mem::take(&mut field_name),
                children: Option::from(if let Some(last_curr_query_part) = last_curr_query_part {
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

          last_possible_fieldname_idx = j;
          j -= 1;
          is_not_allowed = true;
        } else if is_not_allowed
          && !did_encounter_escape
          && query_chars_len > 5 // overflow
          && j < query_chars_len - 4
          && query_chars[j] == 'N' && query_chars[j + 1] == 'O' && query_chars[j + 2] == 'T'
          && query_chars[j + 3].is_ascii_whitespace() {
          handle_terminator(
            tokenizer, &query_chars,
            i, j, &escape_indices,
            &mut query_parts, &mut is_expecting_and, &mut did_encounter_not, &mut field_name
          )?;
          
          did_encounter_not = true;

          j += 4;
          while j < query_chars_len && query_chars[j].is_ascii_whitespace() {
            j += 1;
          }
          i = j;
          last_possible_fieldname_idx = i;
          j -= 1;
        } else if c == '\\' {
          did_encounter_escape = !did_encounter_escape;
          if did_encounter_escape {
            escape_indices.push(j);
          }
        } else {
          did_encounter_escape = false;
          is_not_allowed = false;
        }
      }
    }

    j += 1;
  }

  handle_terminator(tokenizer, &query_chars, i, j, &escape_indices, &mut query_parts, &mut is_expecting_and, &mut did_encounter_not, &mut field_name)?;

  Ok(query_parts)
}
