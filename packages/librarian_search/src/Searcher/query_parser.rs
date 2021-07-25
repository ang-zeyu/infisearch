use crate::tokenize::Tokenizer;
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
  pub isCorrected: bool,
  pub isStopWordRemoved: bool,
  pub shouldExpand: bool,
  pub isExpanded: bool,
  pub originalTerms: Option<Vec<String>>,
  pub terms: Option<Vec<String>>,
  pub typee: QueryPartType,
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
  let mut is_unary_operator_allowed = true;
  let mut did_encounter_not = false;

  let mut i = 0;
  let mut j = 0;

  let wrap_in_not = |query_part: QueryPart, did_encounter_not: &mut bool| -> QueryPart {
    if *did_encounter_not {
      *did_encounter_not = false;
      QueryPart {
        isCorrected: false,
        isStopWordRemoved: false,
        shouldExpand: false,
        isExpanded: false,
        originalTerms: Option::None,
        terms: Option:: None,
        typee: QueryPartType::NOT,
        children: Option::from(vec![query_part]),
      }
    } else {
      query_part
    }
  };

  let handle_free_text = |query_parts: &mut Vec<QueryPart>, chars: &Vec<char>, i: usize, j: usize, did_encounter_not: &mut bool| {
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
          isCorrected: false,
          isStopWordRemoved: false,
          shouldExpand: tokenize_result.should_expand,
          isExpanded: false,
          originalTerms: Option::None,
          terms: Option::from(vec![term]),
          typee: QueryPartType::TERM,
          children: Option::None,
        }, did_encounter_not));
      } else {
        query_parts.push(QueryPart {
          isCorrected: false,
          isStopWordRemoved: false,
          shouldExpand: tokenize_result.should_expand,
          isExpanded: false,
          originalTerms: Option::None,
          terms: Option::from(vec![term]),
          typee: QueryPartType::TERM,
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
          let typee = if terms.len() <= 1 { QueryPartType::TERM } else { QueryPartType::PHRASE };
          let phrase_query_part: QueryPart = wrap_in_not(QueryPart {
            isCorrected: false,
            isStopWordRemoved: false,
            shouldExpand: false,
            isExpanded: false,
            originalTerms: Option::None,
            terms: Option::from(terms),
            typee,
            children: Option::None,
          }, &mut did_encounter_not);

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

          is_unary_operator_allowed = true;
        }
      },
      QueryParseState::PARENTHESES => {
        if c == ')' {
          query_parse_state = QueryParseState::NONE;
          
          let content: String = query_chars[i..j].iter().collect();
          let child_query_part: QueryPart = wrap_in_not(QueryPart {
            isCorrected: false,
            isStopWordRemoved: false,
            shouldExpand: false,
            isExpanded: false,
            originalTerms: Option::None,
            terms: Option::None,
            typee: QueryPartType::BRACKET,
            children: Option::from(parse_query(content, tokenizer)?),
          }, &mut did_encounter_not);

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

          is_unary_operator_allowed = true;
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
                .push(wrap_in_not(curr_query_parts.remove(0), &mut did_encounter_not));
              query_parts.append(&mut curr_query_parts);
              is_expecting_and = false;
            }
            // i === j: the phrase / parentheses is part of the AND (e.g. lorem AND (ipsum))
          } else {
            handle_free_text(&mut query_parts, &query_chars, i, j, &mut did_encounter_not);
          }

          query_parse_state = if c == '"' { QueryParseState::QUOTE } else { QueryParseState::PARENTHESES };
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
              let first_query_part = wrap_in_not(curr_query_parts.swap_remove(0), &mut did_encounter_not);
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
                  isCorrected: false,
                  isStopWordRemoved: false,
                  shouldExpand: false,
                  isExpanded: false,
                  originalTerms: Option::None,
                  terms: Option::None,
                  typee: QueryPartType::AND,
                  children: Option::from(vec![last_curr_query_part]),
                });
              }
            } else if query_parts.len() > 0 && !is_expecting_and {
              // e.g. (lorem) AND ipsum
              let last_curr_query_part = query_parts.pop().unwrap();
              if let QueryPartType::AND = last_curr_query_part.typee {
                // Reuse last AND group
                query_parts.push(last_curr_query_part);
              } else {
                query_parts.push(QueryPart {
                  isCorrected: false,
                  isStopWordRemoved: false,
                  shouldExpand: false,
                  isExpanded: false,
                  originalTerms: Option::None,
                  terms: Option::None,
                  typee: QueryPartType::AND,
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

          j -= 1;
          is_unary_operator_allowed = true;
        } else if is_unary_operator_allowed
          && query_chars_len > 5 // overflow
          && j < query_chars_len - 4
          && query_chars[j] == 'N' && query_chars[j + 1] == 'O' && query_chars[j + 2] == 'T'
          && query_chars[j + 3].is_ascii_whitespace() {
          let mut curr_query_parts = parse_query(query_chars[i..j].iter().collect(), tokenizer)?;
          if curr_query_parts.len() > 0 {
            let first_query_part = wrap_in_not(curr_query_parts.swap_remove(0), &mut did_encounter_not);
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
          is_unary_operator_allowed = false;
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
        .push(wrap_in_not(curr_query_parts.remove(0), &mut did_encounter_not));
      query_parts.append(&mut curr_query_parts);
    } else {
      return Err("Query parsing error: no token found after AND operator");
    }
  } else {
    handle_free_text(&mut query_parts, &query_chars, i, j, &mut did_encounter_not);
  }

  Ok(query_parts)
}
