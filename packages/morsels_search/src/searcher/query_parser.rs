use morsels_common::tokenize::Tokenizer;
use serde::Serialize;

#[derive(Serialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum QueryPartType {
    Term,
    Phrase,
    Bracket,
    And,
    Not,
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

enum Operator {
    Not,
    And,
    OpenGroup,
    Field(String),
}

enum QueryParseState {
    None,
    Quote,
}

fn handle_op(query_parts: &mut Vec<QueryPart>, operator_stack: &mut Vec<Operator>) {
    while !operator_stack.is_empty() {
        let op = operator_stack.pop().unwrap();
        match op {
            Operator::Not => {
                let last_part = query_parts.pop().unwrap();
                query_parts.push(QueryPart {
                    is_corrected: false,
                    is_stop_word_removed: false,
                    should_expand: false,
                    is_expanded: false,
                    original_terms: None,
                    terms: None,
                    part_type: QueryPartType::Not,
                    field_name: None,
                    children: Some(vec![last_part]),
                });
            }
            Operator::And => {
                let last_part = query_parts.pop().unwrap();
                query_parts.last_mut().unwrap().children.as_mut().unwrap().push(last_part);
            }
            Operator::OpenGroup => {
                // Serves as a guard to the rest of the stack.
                // This will only be popped when ')' is encountered.
                operator_stack.push(op);
                return;
            }
            Operator::Field(field_name) => {
                query_parts.last_mut().unwrap().field_name = Some(field_name);
            }
        }
    }
}

fn collect_slice(query_chars: &[char], i: usize, j: usize, escape_indices: &[usize]) -> String {
    query_chars[i..j]
        .iter()
        .enumerate()
        .filter(|(idx, _char)| escape_indices.iter().find(|escape_idx| **escape_idx == (*idx + i)).is_none())
        .map(|(_idx, c)| c)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn handle_terminator(
    tokenizer: &dyn Tokenizer,
    query_chars: &[char],
    i: usize,
    j: usize,
    escape_indices: &[usize],
    query_parts: &mut Vec<QueryPart>,
    operator_stack: &mut Vec<Operator>,
) {
    if i == j {
        return;
    }

    let tokenize_result = tokenizer.wasm_tokenize(collect_slice(query_chars, i, j, &escape_indices));
    if tokenize_result.terms.is_empty() {
        return;
    }

    let mut is_first = true;
    for term in tokenize_result.terms {
        if is_first {
            is_first = false;

            query_parts.push(QueryPart {
                is_corrected: false,
                is_stop_word_removed: false,
                should_expand: tokenize_result.should_expand,
                is_expanded: false,
                original_terms: None,
                terms: Some(vec![term]),
                part_type: QueryPartType::Term,
                field_name: None,
                children: None,
            });
            handle_op(query_parts, operator_stack);
        } else {
            query_parts.push(QueryPart {
                is_corrected: false,
                is_stop_word_removed: false,
                should_expand: tokenize_result.should_expand,
                is_expanded: false,
                original_terms: None,
                terms: Some(vec![term]),
                part_type: QueryPartType::Term,
                field_name: None,
                children: None,
            });
        }
    }
}

pub fn parse_query(query: String, tokenizer: &dyn Tokenizer) -> Vec<QueryPart> {
    let mut query_parts: Vec<QueryPart> = Vec::with_capacity(5);

    let mut query_parse_state: QueryParseState = QueryParseState::None;
    let mut did_encounter_escape = false;
    let mut escape_indices: Vec<usize> = Vec::new();
    let mut op_stack: Vec<Operator> = Vec::new();

    let mut i = 0;
    let mut j = 0;
    let mut last_possible_unaryop_idx = 0;

    let query_chars: Vec<char> = query.chars().collect();
    let query_chars_len = query_chars.len();

    while j < query_chars_len {
        let c = query_chars[j];

        match query_parse_state {
            QueryParseState::Quote => {
                if !did_encounter_escape && c == '"' {
                    let content = collect_slice(&query_chars, i, j, &escape_indices);
                    query_parse_state = QueryParseState::None;

                    query_parts.push(QueryPart {
                        is_corrected: false,
                        is_stop_word_removed: false,
                        should_expand: false,
                        is_expanded: false,
                        original_terms: None,
                        terms: Some(tokenizer.wasm_tokenize(content).terms),
                        part_type: QueryPartType::Phrase,
                        field_name: None,
                        children: None,
                    });
                    handle_op(&mut query_parts, &mut op_stack);

                    i = j + 1;
                    last_possible_unaryop_idx = i;
                } else if c == '\\' {
                    did_encounter_escape = true;
                } else {
                    did_encounter_escape = false;
                }
            }
            QueryParseState::None => {
                if !did_encounter_escape && (c == '"' || c == '(' || c == ')') {
                    handle_terminator(
                        tokenizer,
                        &query_chars,
                        i,
                        j,
                        &escape_indices,
                        &mut query_parts,
                        &mut op_stack,
                    );

                    i = j + 1;

                    match c {
                        '"' => query_parse_state = QueryParseState::Quote,
                        '(' => {
                            query_parts.push(QueryPart {
                                is_corrected: false,
                                is_stop_word_removed: false,
                                should_expand: false,
                                is_expanded: false,
                                original_terms: None,
                                terms: None,
                                part_type: QueryPartType::Bracket,
                                field_name: None,
                                children: None,
                            });
                            op_stack.push(Operator::OpenGroup);
                            last_possible_unaryop_idx = i;
                        }
                        ')' => {
                            if !op_stack.is_empty() && matches!(op_stack.last().unwrap(), Operator::OpenGroup)
                            {
                                let mut children: Vec<QueryPart> = Vec::new();
                                while let Some(mut last_part) = query_parts.pop() {
                                    if let QueryPartType::Bracket = last_part.part_type {
                                        if last_part.children.is_none() {
                                            children.reverse();
                                            last_part.children = Some(children);
                                            query_parts.push(last_part);

                                            op_stack.pop();
                                            handle_op(&mut query_parts, &mut op_stack);
                                            break;
                                        } else {
                                            // Nested parentheses
                                            children.push(last_part);
                                        }
                                    } else {
                                        children.push(last_part);
                                    }
                                }
                            }
                            last_possible_unaryop_idx = i;
                        }
                        _ => {}
                    }
                } else if c == ':' && !did_encounter_escape && last_possible_unaryop_idx >= i && j > i {
                    handle_terminator(
                        tokenizer,
                        &query_chars,
                        i,
                        last_possible_unaryop_idx,
                        &escape_indices,
                        &mut query_parts,
                        &mut op_stack,
                    );

                    op_stack.push(Operator::Field(collect_slice(
                        &query_chars,
                        last_possible_unaryop_idx,
                        j,
                        &escape_indices,
                    )));
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
                        && query_chars[j + 3].is_ascii_whitespace()
                    {
                        handle_terminator(
                            tokenizer,
                            &query_chars,
                            i,
                            initial_j,
                            &escape_indices,
                            &mut query_parts,
                            &mut op_stack,
                        );

                        if query_parts.is_empty()
                            || !matches!(query_parts.last().unwrap().part_type, QueryPartType::And)
                        {
                            let children = Some(if let Some(last_curr_query_part) = query_parts.pop() {
                                vec![last_curr_query_part]
                            } else {
                                vec![]
                            });

                            query_parts.push(QueryPart {
                                is_corrected: false,
                                is_stop_word_removed: false,
                                should_expand: false,
                                is_expanded: false,
                                original_terms: None,
                                terms: None,
                                part_type: QueryPartType::And,
                                field_name: None,
                                children,
                            });
                        }

                        op_stack.push(Operator::And);

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
                    && query_chars[j + 3].is_ascii_whitespace()
                {
                    handle_terminator(
                        tokenizer,
                        &query_chars,
                        i,
                        j,
                        &escape_indices,
                        &mut query_parts,
                        &mut op_stack,
                    );

                    op_stack.push(Operator::Not);

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

    handle_terminator(tokenizer, &query_chars, i, j, &escape_indices, &mut query_parts, &mut op_stack);

    query_parts
}

#[cfg(test)]
pub mod test {
    use pretty_assertions::assert_eq;

    use morsels_lang_ascii::ascii::{self, TokenizerOptions};

    use super::{QueryPart, QueryPartType};

    impl QueryPart {
        fn no_expand(mut self) -> QueryPart {
            if let QueryPartType::Term = self.part_type {
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
            part_type: QueryPartType::Not,
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
            part_type: QueryPartType::And,
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
            part_type: QueryPartType::Bracket,
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
            part_type: QueryPartType::Term,
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

    fn get_phrase(terms: Vec<&str>) -> QueryPart {
        QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: false,
            is_expanded: false,
            original_terms: None,
            terms: Some(terms.into_iter().map(|term| term.to_owned()).collect()),
            part_type: QueryPartType::Phrase,
            field_name: None,
            children: None,
        }
    }

    pub fn parse(query: &str) -> Vec<QueryPart> {
        let tokenizer = ascii::new_with_options(TokenizerOptions {
            stop_words: None,
            max_term_len: 80,
        });

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
        assert_eq!(
            parse("lorem AND ipsum AND lorem"),
            vec![wrap_in_and(vec![get_lorem(), get_ipsum(), get_lorem()])]
        );
        assert_eq!(
            parse("lorem AND NOT ipsum"),
            vec![wrap_in_and(vec![get_lorem(), wrap_in_not(get_ipsum())])]
        );
        assert_eq!(
            parse("NOT lorem AND NOT ipsum"),
            vec![wrap_in_and(vec![wrap_in_not(get_lorem()), wrap_in_not(get_ipsum())])]
        );
        assert_eq!(
            parse("NOT lorem AND NOT ipsum lorem NOT ipsum"),
            vec![
                wrap_in_and(vec![wrap_in_not(get_lorem()), wrap_in_not(get_ipsum().no_expand())]),
                get_lorem().no_expand(),
                wrap_in_not(get_ipsum())
            ]
        );
    }

    #[test]
    fn phrase_test() {
        assert_eq!(parse("\"lorem ipsum\""), vec![get_phrase(vec!["lorem", "ipsum"])]);
        assert_eq!(parse("\"(lorem ipsum)\""), vec![get_phrase(vec!["lorem", "ipsum"])]);
        assert_eq!(parse("lorem\"lorem ipsum\""), vec![get_lorem(), get_phrase(vec!["lorem", "ipsum"])]);
        assert_eq!(
            parse("\"lorem ipsum\"lorem\"lorem ipsum\""),
            vec![get_phrase(vec!["lorem", "ipsum"]), get_lorem(), get_phrase(vec!["lorem", "ipsum"]),]
        );
        assert_eq!(
            parse("\"lorem ipsum\" lorem \"lorem ipsum\""),
            vec![
                get_phrase(vec!["lorem", "ipsum"]),
                get_lorem().no_expand(),
                get_phrase(vec!["lorem", "ipsum"]),
            ]
        );
    }

    #[test]
    fn parentheses_test() {
        assert_eq!(parse("(lorem ipsum)"), vec![wrap_in_parentheses(vec![get_lorem(), get_ipsum()])]);
        assert_eq!(
            parse("(lorem ipsum )"),
            vec![wrap_in_parentheses(vec![get_lorem().no_expand(), get_ipsum().no_expand()])]
        );
        assert_eq!(
            parse("lorem(lorem ipsum)"),
            vec![get_lorem(), wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),]
        );
        assert_eq!(
            parse("(lorem ipsum)lorem(lorem ipsum)"),
            vec![
                wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
                get_lorem(),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
            ]
        );
        assert_eq!(
            parse("(lorem ipsum) lorem (lorem ipsum)"),
            vec![
                wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
                get_lorem().no_expand(),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
            ]
        );
        assert_eq!(
            parse("(lorem ipsum) lorem (lorem ipsum)"),
            vec![
                wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
                get_lorem().no_expand(),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum()]),
            ]
        );
        assert_eq!(
            parse("((lorem ipsum) lorem) (lorem(ipsum))"),
            vec![
                wrap_in_parentheses(vec![wrap_in_parentheses(vec![get_lorem(), get_ipsum()]), get_lorem(),]),
                wrap_in_parentheses(vec![get_lorem(), wrap_in_parentheses(vec![get_ipsum()]),]),
            ]
        );
    }

    #[test]
    fn field_name_test() {
        assert_eq!(parse("title:lorem"), vec![get_lorem().with_field("title")]);
        assert_eq!(parse("title:lorem ipsum"), vec![get_lorem().with_field("title"), get_ipsum()]);
        assert_eq!(
            parse("title:lorem body:ipsum"),
            vec![get_lorem().with_field("title").no_expand(), get_ipsum().with_field("body")]
        );
        assert_eq!(
            parse("title:(lorem body:ipsum)"),
            vec![wrap_in_parentheses(vec![get_lorem().no_expand(), get_ipsum().with_field("body")])
                .with_field("title")]
        );
        assert_eq!(
            parse("title:lorem AND ipsum"),
            vec![wrap_in_and(vec![get_lorem().with_field("title"), get_ipsum()])]
        );
        assert_eq!(
            parse("title:(lorem AND ipsum)"),
            vec![wrap_in_parentheses(vec![wrap_in_and(vec![get_lorem(), get_ipsum()])]).with_field("title")]
        );
        assert_eq!(
            parse("title:NOT lorem ipsum)"),
            vec![wrap_in_not(get_lorem()).with_field("title"), get_ipsum()]
        );
        assert_eq!(
            parse("title: NOT lorem ipsum)"),
            vec![wrap_in_not(get_lorem()).with_field("title"), get_ipsum()]
        );
        assert_eq!(
            parse("title: lorem NOT ipsum)"),
            vec![get_lorem().with_field("title").no_expand(), wrap_in_not(get_ipsum())]
        );
    }

    #[test]
    fn misc_test() {
        assert_eq!(
            parse("title:(lorem AND ipsum) AND NOT (lorem ipsum) body:(lorem NOT ipsum)"),
            vec![
                wrap_in_and(vec![
                    wrap_in_parentheses(vec![wrap_in_and(vec![get_lorem(), get_ipsum()])])
                        .with_field("title"),
                    wrap_in_not(wrap_in_parentheses(vec![get_lorem(), get_ipsum(),]))
                ]),
                wrap_in_parentheses(vec![get_lorem().no_expand(), wrap_in_not(get_ipsum())])
                    .with_field("body")
            ]
        );

        assert_eq!(
            parse("title:(lorem AND ipsum)NOT title:(lorem ipsum) body:(lorem NOT ipsum)"),
            vec![
                wrap_in_parentheses(vec![wrap_in_and(vec![get_lorem(), get_ipsum()])]).with_field("title"),
                wrap_in_not(wrap_in_parentheses(vec![get_lorem(), get_ipsum(),]).with_field("title")),
                wrap_in_parentheses(vec![get_lorem().no_expand(), wrap_in_not(get_ipsum())])
                    .with_field("body")
            ]
        );

        assert_eq!(
            parse("title:\"lorem AND ipsum\"NOT title:(\"lorem ipsum\") body:(lorem NOT ipsum)"),
            vec![
                get_phrase(vec!["lorem", "and", "ipsum"]).with_field("title"),
                wrap_in_not(
                    wrap_in_parentheses(vec![get_phrase(vec!["lorem", "ipsum"])]).with_field("title")
                ),
                wrap_in_parentheses(vec![get_lorem().no_expand(), wrap_in_not(get_ipsum())])
                    .with_field("body")
            ]
        );

        assert_eq!(
            parse("title:(lorem AND body:(lorem ipsum))NOT title:((body:\"lorem\") ipsum) body:(lorem NOT ipsum)"),
            vec![
                wrap_in_parentheses(vec![
                    wrap_in_and(vec![
                        get_lorem(),
                        wrap_in_parentheses(vec![
                            get_lorem(),
                            get_ipsum(),
                        ]).with_field("body"),
                    ])
                ]).with_field("title"),
                wrap_in_not(wrap_in_parentheses(vec![
                    wrap_in_parentheses(vec![
                        get_phrase(vec!["lorem"]).with_field("body"),
                    ]),
                    get_ipsum(),
                ]).with_field("title")),
                wrap_in_parentheses(vec![get_lorem().no_expand(), wrap_in_not(get_ipsum())]).with_field("body")
            ]
        );

        assert_eq!(
            parse("title:lorem AND ipsum AND NOT lorem ipsum body:lorem NOT ipsum"),
            vec![
                wrap_in_and(vec![
                    get_lorem().with_field("title"),
                    get_ipsum(),
                    wrap_in_not(get_lorem().no_expand()),
                ]),
                get_ipsum().no_expand(),
                get_lorem().no_expand().with_field("body"),
                wrap_in_not(get_ipsum()),
            ]
        );

        assert_eq!(
            parse("title\\:lorem\\ AND ipsum\\ AND \\NOT lorem ipsum body\\:lorem \\NOT ipsum"),
            vec![
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
            ]
        );
    }
}
