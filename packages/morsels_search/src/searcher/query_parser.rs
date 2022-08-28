use morsels_common::{
    tokenize::{SearchTokenizer, SearchTokenizeTerm},
    dictionary::Dictionary,
};

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub enum QueryPartType {
    Term,
    Phrase,
    Bracket,
    And,
    Not,
}

#[cfg_attr(test, derive(Debug))]
pub struct QueryPart {
    pub is_corrected: bool,
    pub is_stop_word_removed: bool,
    pub should_expand: bool,
    pub is_expanded: bool,
    pub original_terms: Option<Vec<String>>,
    pub terms: Option<Vec<String>>,
    pub terms_searched: Option<Vec<Vec<String>>>,
    pub part_type: QueryPartType,
    pub field_name: Option<String>,
    pub children: Option<Vec<QueryPart>>,
    pub include_in_proximity_ranking: bool,
    pub weight: f32,
}

#[cfg(test)]
impl Eq for QueryPart {}

#[cfg(test)]
impl PartialEq for QueryPart {
    fn eq(&self, other: &Self) -> bool {
        self.is_corrected == other.is_corrected
            && self.is_stop_word_removed == other.is_stop_word_removed
            && self.should_expand == other.should_expand
            && self.is_expanded == other.is_expanded
            && self.original_terms == other.original_terms
            && self.terms == other.terms
            && self.terms_searched == other.terms_searched
            && self.part_type == other.part_type
            && self.field_name == other.field_name
            && self.children == other.children
            && self.include_in_proximity_ranking == other.include_in_proximity_ranking
            && (self.weight - other.weight).abs() < 0.001
    }
}

#[inline(never)]
fn wrap_string(s: &String) -> String {
    let mut output = String::with_capacity(s.len() + 2);
    output.push('"');

    for c in s.chars() {
        match c {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            _ => output.push(c),
        }
    }

    output.push('"');
    output
}

#[inline(never)]
fn serialize_bool(name: &str, b: bool, output: &mut String) {
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(if b { "true," } else { "false," });
}

#[inline(never)]
fn get_null() -> String {
    "null".to_owned()
}

#[inline(never)]
pub fn serialize_string_vec(v: &Vec<String>) -> String {
    let mut output = "[".to_owned();
    let wrapped: Vec<String> = v.iter().map(wrap_string).collect();
    output.push_str(wrapped.join(",").as_str());
    output.push(']');
    output
}

impl QueryPart {
    #[inline(never)]
    pub fn serialize_parts(parts: &Vec<QueryPart>) -> String {
        let mut output = "[".to_owned();
        let wrapped: Vec<String> = parts.iter().map(QueryPart::serialize).collect();
        output.push_str(wrapped.join(",").as_str());
        output.push(']');
        output
    }

    fn serialize(&self) -> String {
        let mut output = "{".to_owned();

        serialize_bool("isCorrected", self.is_corrected, &mut output);
        serialize_bool("isStopWordRemoved", self.is_stop_word_removed, &mut output);
        serialize_bool("shouldExpand", self.should_expand, &mut output);
        serialize_bool("isExpanded", self.is_expanded, &mut output);

        output.push_str(r#""originalTerms":"#);
        output.push_str(&if let Some(v) = &self.original_terms {
            serialize_string_vec(v)
        } else {
            get_null()
        });

        output.push_str(r#","terms":"#);
        output.push_str(&if let Some(v) = &self.terms {
            serialize_string_vec(v)
        } else {
            get_null()
        });

        output.push_str(r#","partType":"#);
        output.push_str(match self.part_type {
            QueryPartType::Term => "\"TERM\"",
            QueryPartType::Phrase => "\"PHRASE\"",
            QueryPartType::Bracket => "\"BRACKET\"",
            QueryPartType::And => "\"AND\"",
            QueryPartType::Not => "\"NOT\"",
        });

        output.push_str(r#","fieldName":"#);
        output.push_str(&if let Some(v) = &self.field_name {
            wrap_string(v)
        } else {
            get_null()
        });

        output.push_str(r#","children":"#);
        output.push_str(&if let Some(children) = &self.children {
            Self::serialize_parts(children)
        } else {
            get_null()
        });

        output.push('}');
        output
    }

    fn get_base(part_type: QueryPartType) -> Self {
        QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: false,
            is_expanded: false,
            original_terms: None,
            terms: None,
            terms_searched: None,
            part_type,
            field_name: None,
            children: None,
            weight: 1.0,
            include_in_proximity_ranking: true,
        }
    }
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

/// Called whenever a new QueryPart is added into query_parts
#[inline(never)]
fn handle_op(query_parts: &mut Vec<QueryPart>, operator_stack: &mut Vec<Operator>) {
    while let Some(op) = operator_stack.pop() {
        match op {
            Operator::Not => {
                let last_part = query_parts.pop().unwrap();
                query_parts.push(QueryPart {
                    children: Some(vec![last_part]),
                    include_in_proximity_ranking: false,
                    ..QueryPart::get_base(QueryPartType::Not)
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
                let last = query_parts.last_mut().unwrap();
                if last.field_name.is_none() {
                    last.field_name = Some(field_name);
                }
            }
        }
    }
}

#[inline(never)]
fn collect_slice(query_chars: &[char], i: usize, j: usize, escape_indices: &[usize]) -> String {
    query_chars[i..j]
        .iter()
        .enumerate()
        .filter(|(idx, _char)| !escape_indices.iter().any(|escape_idx| *escape_idx == (*idx + i)))
        .map(|(_idx, c)| c)
        .collect()
}

#[allow(clippy::match_like_matches_macro)]
#[inline(never)]
fn is_double_quote(c: char) -> bool {
    match c {
        '"' |
        '″' |
        '‶' |
        '“' |
        '”' |
        '❝' |
        '❞' |
        '＂'
        => true,
        _ => false
    }
}

#[inline(never)]
fn is_ascii_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' ')
}

// TODO cleanup query parsing, hopefully reduce its size while at it. Currently ~7.5KB.

/// Called when 1 of the operators: NOT, AND, (, ), ", :, is encountered
/// or at the end of input
/// 
/// Tokenizes the current slice into term query parts,
/// and calls handle_op for the first term, if required. 
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn handle_terminator(
    tokenizer: &dyn SearchTokenizer,
    query_chars: &[char],
    i: usize,
    j: usize,
    escape_indices: &[usize],
    query_parts: &mut Vec<QueryPart>,
    operator_stack: &mut Vec<Operator>,
    dict: &Dictionary,
) {
    if i == j {
        return;
    }

    let tokenize_result = tokenizer.search_tokenize(collect_slice(query_chars, i, j, escape_indices), dict);
    if tokenize_result.terms.is_empty() {
        return;
    }

    let mut is_first = true;
    for SearchTokenizeTerm {
        term,
        term_inflections,
        original_term,
    } in tokenize_result.terms {
        query_parts.push(QueryPart {
            should_expand: tokenize_result.should_expand,
            terms: term.map(|t| vec![t]),
            original_terms: Some(vec![original_term]),
            terms_searched: Some(vec![term_inflections]),
            ..QueryPart::get_base(QueryPartType::Term)
        });

        if is_first {
            is_first = false;
            handle_op(query_parts, operator_stack);
        }
    }
}

pub fn parse_query(
    query: String,
    tokenizer: &dyn SearchTokenizer,
    valid_fields: &Vec<&str>,
    with_positions: bool,
    dict: &Dictionary,
) -> Vec<QueryPart> {
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
                if !did_encounter_escape && is_double_quote(c) {
                    let content = collect_slice(&query_chars, i, j, &escape_indices);
                    query_parse_state = QueryParseState::None;

                    let tokenize_result = tokenizer.search_tokenize(content, dict);

                    let mut terms = Vec::new();
                    let mut terms_searched = Vec::new();

                    for SearchTokenizeTerm {
                        term,
                        term_inflections,
                        original_term: _,
                    } in tokenize_result.terms {
                        if let Some(term) = term {
                            terms.push(term);
                        }

                        terms_searched.push(term_inflections);
                    }

                    query_parts.push(QueryPart {
                        terms: Some(terms),
                        terms_searched: Some(terms_searched),
                        ..QueryPart::get_base(QueryPartType::Phrase)
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
                if !did_encounter_escape && ((with_positions && is_double_quote(c)) || c == '(' || c == ')') {
                    handle_terminator(
                        tokenizer,
                        &query_chars,
                        i,
                        j,
                        &escape_indices,
                        &mut query_parts,
                        &mut op_stack,
                        dict,
                    );

                    i = j + 1;

                    if is_double_quote(c) {
                        query_parse_state = QueryParseState::Quote;
                    } else if c == '(' {
                        query_parts.push(QueryPart::get_base(QueryPartType::Bracket));
                        op_stack.push(Operator::OpenGroup);
                        last_possible_unaryop_idx = i;
                    } else if c == ')' {
                        // Guard against ')' without a matching '(' (just treat it literally, almost)
                        if !op_stack.is_empty() && matches!(op_stack.last().unwrap(), Operator::OpenGroup)
                        {
                            // Keep going until we find the QueryPartType::Bracket added by '('
                            let open_bracket_querypart_idx = query_parts
                                .iter()
                                .enumerate()
                                .rev()
                                .find_map(|(idx, query_part)|
                                    if matches!(query_part.part_type, QueryPartType::Bracket)
                                        && query_part.children.is_none()
                                    {
                                        Some(idx)
                                    } else {
                                        None
                                    }
                                );
                            
                            if let Some(idx) = open_bracket_querypart_idx {
                                let children: Vec<QueryPart> = query_parts.drain(idx + 1..).collect();
                                query_parts.last_mut().unwrap().children = Some(children);

                                op_stack.pop(); // throw the OpenGroup
                                handle_op(&mut query_parts, &mut op_stack);
                            }
                        }
                        last_possible_unaryop_idx = i;
                    }
                } else if c == ':' && !did_encounter_escape && last_possible_unaryop_idx >= i && j > i {
                    let field_name = collect_slice(
                        &query_chars,
                        last_possible_unaryop_idx,
                        j,
                        &escape_indices,
                    );

                    // Treat it literally otherwise
                    if valid_fields.contains(&field_name.as_str()) {
                        handle_terminator(
                            tokenizer,
                            &query_chars,
                            i,
                            last_possible_unaryop_idx,
                            &escape_indices,
                            &mut query_parts,
                            &mut op_stack,
                            dict,
                        );

                        op_stack.push(Operator::Field(field_name));
                        i = j + 1;
                        last_possible_unaryop_idx = i;
                    }
                } else if is_ascii_whitespace(c) {
                    let initial_j = j;
                    while j < query_chars_len && is_ascii_whitespace(query_chars[j]) {
                        j += 1;
                    }

                    if !did_encounter_escape
                        && query_chars_len > 6 // overflow
                        &&  j < query_chars_len - 4
                        && query_chars[j] == 'A' && query_chars[j + 1] == 'N' && query_chars[j + 2] == 'D'
                        && is_ascii_whitespace(query_chars[j + 3])
                    {
                        handle_terminator(
                            tokenizer,
                            &query_chars,
                            i,
                            initial_j,
                            &escape_indices,
                            &mut query_parts,
                            &mut op_stack,
                            dict,
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
                                children,
                                ..QueryPart::get_base(QueryPartType::And)
                            });
                        }

                        op_stack.push(Operator::And);

                        j += 4;
                        while j < query_chars_len && is_ascii_whitespace(query_chars[j]) {
                            j += 1;
                        }
                        i = j;
                    }

                    last_possible_unaryop_idx = j;
                    j -= 1;
                } else if j == last_possible_unaryop_idx
                    && !did_encounter_escape
                    && query_chars_len > 5 // overflow guard
                    && j < query_chars_len - 4
                    && query_chars[j] == 'N' && query_chars[j + 1] == 'O' && query_chars[j + 2] == 'T'
                    && is_ascii_whitespace(query_chars[j + 3])
                {
                    handle_terminator(
                        tokenizer,
                        &query_chars,
                        i,
                        j,
                        &escape_indices,
                        &mut query_parts,
                        &mut op_stack,
                        dict,
                    );

                    op_stack.push(Operator::Not);

                    j += 4;
                    while j < query_chars_len && is_ascii_whitespace(query_chars[j]) {
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

    handle_terminator(
        tokenizer,
        &query_chars,
        i,
        j,
        &escape_indices,
        &mut query_parts,
        &mut op_stack,
        dict,
    );

    query_parts
}

#[cfg(test)]
pub mod test {
    use std::collections::BTreeMap;

    use morsels_common::{MorselsLanguageConfig, MorselsLanguageConfigOpts, dictionary::{Dictionary, TermInfo}};
    use pretty_assertions::assert_eq;

    use morsels_lang_ascii::ascii;
    use smartstring::{SmartString, LazyCompact};

    use super::{QueryPart, QueryPartType};

    impl QueryPart {
        fn no_expand(mut self) -> QueryPart {
            if let QueryPartType::Term = self.part_type {
                self.should_expand = false;
                self
            } else {
                panic!("Tried to call no_expand test function on non-term query part");
            }
        }

        fn no_term(mut self) -> QueryPart {
            if matches!(self.part_type, QueryPartType::Term) && self.terms.is_some() {
                self.terms = None;
                self
            } else {
                panic!("Tried to call no_term test function on non-term query part");
            }
        }

        fn with_searched_terms(mut self, terms_searched: Vec<Vec<&str>>) -> QueryPart {
            if self.terms.is_some() {
                self.terms_searched = Some(
                    terms_searched.into_iter()
                        .map(|inner_vec| inner_vec.into_iter().map(|s| s.to_owned()).collect())
                        .collect()
                );
                self
            } else {
                panic!("Tried to call with_searched_terms test function on query part with no terms");
            }
        }

        fn with_field(mut self, field_name: &str) -> QueryPart {
            self.field_name = Some(field_name.to_owned());
            self
        }
    }

    fn get_dictionary() -> Dictionary {
        static TERM_INFO: TermInfo = TermInfo {
            doc_freq: 1,
            postings_file_name: 0,
            postings_file_offset: 65535,
        };

        let mut term_infos: BTreeMap<SmartString<LazyCompact>, &'static TermInfo> = BTreeMap::default();

        for term in vec![
            "lorem", "ipsum", "for", "by", "and", "notipsum", "http", "localhost",
            "8080", "title", "body", "not", "invalidfield",
        ] {
            term_infos.insert(
                SmartString::from(term),
                &TERM_INFO,
            );
        }

        Dictionary { term_infos }
    }

    fn wrap_in_not(query_part: QueryPart) -> QueryPart {
        QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: false,
            is_expanded: false,
            original_terms: None,
            terms: None,
            terms_searched: None,
            part_type: QueryPartType::Not,
            field_name: None,
            children: Some(vec![query_part]),
            weight: 1.0,
            include_in_proximity_ranking: false,
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
            terms_searched: None,
            part_type: QueryPartType::And,
            field_name: None,
            children: Some(query_parts),
            weight: 1.0,
            include_in_proximity_ranking: true,
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
            terms_searched: None,
            part_type: QueryPartType::Bracket,
            field_name: None,
            children: Some(query_parts),
            weight: 1.0,
            include_in_proximity_ranking: true,
        }
    }

    fn get_term(term: &str) -> QueryPart {
        QueryPart {
            is_corrected: false,
            is_stop_word_removed: false,
            should_expand: true,
            is_expanded: false,
            original_terms: Some(vec![term.to_owned()]),
            terms: Some(vec![term.to_owned()]),
            terms_searched: Some(vec![vec![term.to_owned()]]),
            part_type: QueryPartType::Term,
            field_name: None,
            children: None,
            weight: 1.0,
            include_in_proximity_ranking: true,
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
            terms: Some(terms.iter().map(|&term| term.to_owned()).collect()),
            terms_searched: Some(terms.iter().map(|&term| vec![term.to_owned()]).collect()),
            part_type: QueryPartType::Phrase,
            field_name: None,
            children: None,
            weight: 1.0,
            include_in_proximity_ranking: true,
        }
    }

    pub fn parse(query: &str) -> Vec<QueryPart> {
        let tokenizer = ascii::new_with_options(&MorselsLanguageConfig {
            lang: "ascii".to_owned(),
            options: MorselsLanguageConfigOpts::default(),
        });

        super::parse_query(
            query.to_owned(),
            &tokenizer,
            &vec!["title", "body"],
            true,
            &get_dictionary(),
        )
    }

    pub fn parse_wo_pos(query: &str) -> Vec<QueryPart> {
        let tokenizer = ascii::new_with_options(&MorselsLanguageConfig {
            lang: "latin".to_owned(),
            options: MorselsLanguageConfigOpts::default(),
        });

        super::parse_query(
            query.to_owned(),
            &tokenizer,
            &vec!["title", "body"],
            false,
            &get_dictionary(),
        )
    }

    // The tokenizer will remove stop words if they are not even indexed
    pub fn parse_with_sw_removal(query: &str) -> Vec<QueryPart> {
        let tokenizer = ascii::new_with_options(&MorselsLanguageConfig {
            lang: "ascii".to_owned(),
            options: MorselsLanguageConfigOpts {
                stop_words: None,
                ignore_stop_words: Some(true),
                stemmer: None,
                max_term_len: None,
            },
        });

        super::parse_query(
            query.to_owned(),
            &tokenizer,
            &vec!["title", "body"],
            true,
            &get_dictionary(),
        )
    }

    #[test]
    fn free_text_test() {
        assert_eq!(parse("lorem ipsum"), vec![get_lorem(), get_ipsum()]);
        assert_eq!(parse("lorem ipsum "), vec![get_lorem().no_expand(), get_ipsum().no_expand()]);
        assert_eq!(parse_with_sw_removal("for by lorem and"), vec![
            get_term("for").no_term(), get_term("by").no_term(),
            get_lorem(), get_term("and").no_term(),
        ]);
    }

    #[test]
    fn boolean_test() {
        assert_eq!(parse("NOT "), vec![get_term("not").no_expand()]);
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
        assert_eq!(parse_with_sw_removal("for AND by"), vec![wrap_in_and(vec![
            get_term("for").no_term(), get_term("by").no_term(),
        ])]);
        assert_eq!(parse_with_sw_removal("for AND lorem"), vec![wrap_in_and(vec![
            get_term("for").no_term(), get_lorem()
        ])]);
    }

    #[test]
    fn phrase_test() {
        assert_eq!(parse_wo_pos("\"lorem ipsum\""), vec![get_term("lorem"), get_term("ipsum")]);

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
        assert_eq!(
            parse_with_sw_removal("\"for by lorem and\""),
            vec![
                get_phrase(vec!["lorem"]).with_searched_terms(vec![
                    vec!["for"], vec!["by"], vec!["lorem"], vec!["and"],
                ])
            ]
        );
        assert_eq!(
            parse_with_sw_removal("\"l'orem for by ipsum and\""),
            vec![
                get_phrase(vec!["lorem", "ipsum"]).with_searched_terms(vec![
                    vec!["l'orem", "l’orem", "lorem"], vec!["for"], vec!["by"],
                    vec!["ipsum"], vec!["and"],
                ])
            ]
        );
        assert_eq!(
            parse_with_sw_removal("\"lorem ipsum\""),
            vec![
                get_phrase(vec!["lorem", "ipsum"]),
            ]
        );
    }

    #[test]
    fn parentheses_test() {
        // assert_eq!(parse("(lorem ipsum"), vec![get_lorem(), get_ipsum()]);

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
        assert_eq!(
            parse("((lorem ipsum) lorem) (lorem()ipsum)"),
            vec![
                wrap_in_parentheses(vec![wrap_in_parentheses(vec![get_lorem(), get_ipsum()]), get_lorem(),]),
                wrap_in_parentheses(vec![get_lorem(), wrap_in_parentheses(vec![]), get_ipsum()]),
            ]
        );
        assert_eq!(
            parse_with_sw_removal("(for and lorem by)"),
            vec![wrap_in_parentheses(vec![
                get_term("for").no_term(), get_term("and").no_term(), get_lorem(), get_term("by").no_term(),
            ])]
        );
    }

    #[test]
    fn field_name_test() {
        assert_eq!(parse("title:lorem"), vec![get_lorem().with_field("title")]);
        assert_eq!(parse("title:lorem ipsum"), vec![get_lorem().with_field("title"), get_ipsum()]);
        assert_eq!(parse("lorem title: "), vec![get_lorem().no_expand()]);
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
        assert_eq!(
            parse_with_sw_removal("title:for)"),
            vec![get_term("for").with_field("title").no_term()]
        );
        assert_eq!(
            parse_with_sw_removal("title:for body:lorem"),
            vec![
                get_term("for").with_field("title").no_expand().no_term(),
                get_lorem().with_field("body"),
            ]
        );
        assert_eq!(
            parse_with_sw_removal("title:lorem body:for"),
            vec![
                get_lorem().with_field("title").no_expand(),
                get_term("for").with_field("body").no_term(),
            ],
        );

        // Test invalid field names (should be parsed verbose / as-is)
        assert_eq!(
            parse("invalidfield: lorem NOT ipsum)"),
            vec![
                get_term("invalidfield").no_expand(),
                get_lorem().no_expand(),
                wrap_in_not(get_ipsum())
            ]
        );
        assert_eq!(
            parse("http://localhost:8080 lorem"),
            vec![
                get_term("http"),
                get_term("localhost"),
                get_term("8080"),
                get_lorem(),
            ]
        );
        assert_eq!(
            parse("http://localhost:8080 NOT lorem"),
            vec![
                get_term("http").no_expand(),
                get_term("localhost").no_expand(),
                get_term("8080").no_expand(),
                wrap_in_not(get_lorem()),
            ]
        );
        assert_eq!(
            parse("http://localhost:8080 title:lorem"),
            vec![
                get_term("http").no_expand(),
                get_term("localhost").no_expand(),
                get_term("8080").no_expand(),
                get_lorem().with_field("title"),
            ]
        );
        assert_eq!(
            parse("body:ipsum http://localhost:8080 title:lorem"),
            vec![
                get_ipsum().with_field("body").no_expand(),
                get_term("http").no_expand(),
                get_term("localhost").no_expand(),
                get_term("8080").no_expand(),
                get_lorem().with_field("title"),
            ]
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
            parse("body:ipsum AND http://localhost:8080 AND NOT (title:lorem)"),
            vec![
                wrap_in_and(vec![
                    get_ipsum().with_field("body"),
                    get_term("http"),
                ]),
                get_term("localhost"),
                wrap_in_and(vec![
                    get_term("8080"),
                    wrap_in_not(wrap_in_parentheses(vec![get_lorem().with_field("title")])),
                ])
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
                get_term("title"),
                get_term("lorem"),
                get_term("and"),
                get_term("ipsum"),
                get_term("and"),
                get_term("not"),
                get_term("lorem"),
                get_term("ipsum"),
                get_term("body"),
                get_term("lorem"),
                get_term("not"),
                get_term("ipsum"),
            ]
        );

        assert_eq!(
            parse("((lorem ipsum) lorem) (lorem()NOT ipsum)"),
            vec![
                wrap_in_parentheses(vec![wrap_in_parentheses(vec![get_lorem(), get_ipsum()]), get_lorem(),]),
                wrap_in_parentheses(vec![get_lorem(), wrap_in_parentheses(vec![]), wrap_in_not(get_ipsum())]),
            ]
        );
    }
}
