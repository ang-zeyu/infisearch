use infisearch_common::{
    tokenize::{self, SearchTokenizer, SearchTokenizeTerm, PrefixResult},
    dictionary::Dictionary,
};

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub enum QueryPartType {
    Term,
    Phrase,
    Bracket,
}

#[cfg_attr(test, derive(Debug))]
pub struct QueryPart {
    // --------------------------------
    // Operators
    pub is_mandatory: bool,
    pub is_subtracted: bool,
    pub is_inverted: bool,

    pub field_name: Option<String>,

    pub suffix_wildcard: bool,
    // --------------------------------

    pub is_corrected: bool,
    pub auto_suffix_wildcard: bool,
    pub is_suffixed: bool,
    pub original_term: Option<String>,
    pub term: Option<String>,
    pub terms_searched: Option<Vec<String>>,
    pub part_type: QueryPartType,
    pub children: Option<Vec<QueryPart>>,
    pub weight: f32,
}

#[cfg(test)]
impl Eq for QueryPart {}

#[cfg(test)]
impl PartialEq for QueryPart {
    fn eq(&self, other: &Self) -> bool {
        self.is_mandatory == other.is_mandatory
            && self.is_subtracted == other.is_subtracted
            && self.is_inverted == other.is_inverted
            && self.is_corrected == other.is_corrected
            && self.auto_suffix_wildcard == other.auto_suffix_wildcard
            && self.suffix_wildcard == other.suffix_wildcard
            && self.is_suffixed == other.is_suffixed
            && self.original_term == other.original_term
            && self.term == other.term
            && self.terms_searched == other.terms_searched
            && self.part_type == other.part_type
            && self.field_name == other.field_name
            && self.children == other.children
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

        serialize_bool("isMandatory", self.is_mandatory, &mut output);
        serialize_bool("isSubtracted", self.is_subtracted, &mut output);
        serialize_bool("isInverted", self.is_inverted, &mut output);
        serialize_bool("isCorrected", self.is_corrected, &mut output);
        serialize_bool("autoSuffixWildcard", self.auto_suffix_wildcard, &mut output);
        serialize_bool("suffixWildcard", self.suffix_wildcard, &mut output);
        serialize_bool("isSuffixed", self.is_suffixed, &mut output);

        output.push_str(r#""originalTerm":"#);
        output.push_str(&if let Some(v) = &self.original_term {
            wrap_string(v)
        } else {
            get_null()
        });

        output.push_str(r#","term":"#);
        output.push_str(&if let Some(v) = &self.term {
            wrap_string(v)
        } else {
            get_null()
        });

        output.push_str(r#","termsSearched":"#);
        output.push_str(&if let Some(v) = &self.terms_searched {
            serialize_string_vec(v)
        } else {
            get_null()
        });

        output.push_str(r#","partType":"#);
        output.push_str(match self.part_type {
            QueryPartType::Term => "\"TERM\"",
            QueryPartType::Phrase => "\"PHRASE\"",
            QueryPartType::Bracket => "\"BRACKET\"",
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

    pub fn get_base(part_type: QueryPartType) -> Self {
        QueryPart {
            is_mandatory: false,
            is_subtracted: false,
            is_inverted: false,
            is_corrected: false,
            auto_suffix_wildcard: false,
            suffix_wildcard: false,
            is_suffixed: false,
            original_term: None,
            term: None,
            terms_searched: None,
            part_type,
            field_name: None,
            children: None,
            weight: 1.0,
        }
    }
}

enum Operator {
    OpenGroup,
    Field {
        field_name: String,
        prefix_ops: PrefixResult,
    },
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
            Operator::OpenGroup => {
                // Serves as a guard to the rest of the stack.
                // This will only be popped when ')' is encountered.
                operator_stack.push(op);
                return;
            }
            Operator::Field {
                field_name,
                prefix_ops,
            } => {
                if let Some(last) = query_parts.last_mut() {
                    if last.field_name.is_none() {
                        last.field_name = Some(field_name);
                        set_prefix_ops(prefix_ops, last);
                    }
                }
            }
        }
    }
}

#[inline(never)]
fn set_prefix_ops(prefix_ops: PrefixResult, part: &mut QueryPart) {
    if !part.is_mandatory && !part.is_subtracted {
        part.is_mandatory = prefix_ops.is_mandatory;
        part.is_subtracted = prefix_ops.is_subtracted;
    }

    if prefix_ops.is_inverted {
        part.is_inverted = true;
    }
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

    let tokenize_result = tokenizer.search_tokenize(
        &query_chars,
        i,
        j,
        escape_indices,
        dict,
    );
    if tokenize_result.terms.is_empty() {
        return;
    }

    let mut is_first = true;
    for SearchTokenizeTerm {
        term,
        term_inflections,
        original_term,
        suffix_wildcard,
        is_corrected,
        prefix_ops,
    } in tokenize_result.terms {
        let mut part = QueryPart {
            auto_suffix_wildcard: tokenize_result.auto_suffix_wildcard,
            suffix_wildcard,
            is_corrected,
            term,
            original_term: Some(original_term),
            terms_searched: Some(term_inflections),
            ..QueryPart::get_base(QueryPartType::Term)
        };
        set_prefix_ops(prefix_ops, &mut part);
        query_parts.push(part);

        if is_first {
            is_first = false;
            handle_op(query_parts, operator_stack);
        }
    }
}

pub fn parse_query(
    query: String,
    tokenizer: &dyn SearchTokenizer,
    valid_fields: &Vec<String>,
    with_positions: bool,
    dict: &Dictionary,
) -> Vec<QueryPart> {
    let mut query_parts: Vec<QueryPart> = Vec::with_capacity(5);

    let mut query_parse_state: QueryParseState = QueryParseState::None;
    let mut did_encounter_escape = false;
    let mut escape_indices: Vec<usize> = Vec::new();
    let mut op_stack: Vec<Operator> = Vec::new();

    let mut i = 0; // start of current slice
    let mut j = 0; // end of current slice
    let mut k = 0;   // end of previous slice / last possible position of a prefix operator

    let query_chars: Vec<char> = query.chars().collect();
    let query_chars_len = query_chars.len();

    while j < query_chars_len {
        let c = unsafe { *query_chars.get_unchecked(j) };

        match query_parse_state {
            QueryParseState::Quote => {
                if !did_encounter_escape && is_double_quote(c) {
                    query_parse_state = QueryParseState::None;

                    let tokenize_result = tokenizer.search_tokenize(
                        &query_chars,
                        i,
                        j,
                        &escape_indices,
                        dict,
                    );

                    let mut children = Vec::new();

                    for SearchTokenizeTerm {
                        term,
                        term_inflections,
                        is_corrected,
                        original_term,
                        suffix_wildcard: _, // TODO unsupported for now
                        prefix_ops: _,
                    } in tokenize_result.terms {
                        children.push(QueryPart {
                            is_mandatory: term.is_some(),
                            is_corrected,
                            term,
                            original_term: Some(original_term),
                            terms_searched: Some(term_inflections),
                            ..QueryPart::get_base(QueryPartType::Term)
                        });
                    }

                    let mut phrase_part = QueryPart {
                        children: Some(children),
                        ..QueryPart::get_base(QueryPartType::Phrase)
                    };
                    let prefix_ops = tokenize::get_prefix_ops(
                        i, 2, k, &query_chars, &escape_indices, tokenizer,
                    );
                    set_prefix_ops(prefix_ops, &mut phrase_part);

                    query_parts.push(phrase_part);
                    handle_op(&mut query_parts, &mut op_stack);

                    i = j + 1;
                    k = i;
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
                        let mut part = QueryPart::get_base(QueryPartType::Bracket);
                        let prefix_ops = tokenize::get_prefix_ops(
                            j, 1, k, &query_chars, &escape_indices, tokenizer,
                        );
                        set_prefix_ops(prefix_ops, &mut part);
                        
                        query_parts.push(part);
                        op_stack.push(Operator::OpenGroup);
                    } else if c == ')' {
                        // Guard against ')' without a matching '(' (just treat it literally, almost)
                        if !op_stack.is_empty() && matches!(unsafe { op_stack.last().unwrap_unchecked() }, Operator::OpenGroup)
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
                                unsafe { query_parts.last_mut().unwrap_unchecked() }.children = Some(children);

                                op_stack.pop(); // throw the OpenGroup
                                handle_op(&mut query_parts, &mut op_stack);
                                k = j + 1;
                            }
                        }
                    }
                } else if !did_encounter_escape && c == ':' {
                    for field_name in valid_fields {
                        if j >= field_name.len() {
                            let field_name_start = j - field_name.len();
                            let text = unsafe { query_chars.get_unchecked(field_name_start..j) }.iter().collect();
    
                            // Treat it literally otherwise
                            if field_name == &text {
                                handle_terminator(
                                    tokenizer,
                                    &query_chars,
                                    i,
                                    field_name_start,
                                    &escape_indices,
                                    &mut query_parts,
                                    &mut op_stack,
                                    dict,
                                );
    
                                let prefix_ops = tokenize::get_prefix_ops(
                                    field_name_start, 1, k, &query_chars, &escape_indices, tokenizer,
                                );
                                op_stack.push(Operator::Field {
                                    field_name: text,
                                    prefix_ops,
                                });
                                i = j + 1;
                                k = i;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if did_encounter_escape {
            did_encounter_escape = false;
        } else if c == '\\' {
            escape_indices.push(j);
            did_encounter_escape = true;
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

    use infisearch_common::language::{InfiLanguageConfig, InfiLanguageConfigOpts};
    use infisearch_common::dictionary::{Dictionary, TermInfo};
    use pretty_assertions::assert_eq;

    use infisearch_lang_ascii::ascii;
    use infisearch_lang_chinese::chinese;
    use smartstring::{SmartString, LazyCompact};

    use super::{QueryPart, QueryPartType};

    impl QueryPart {
        fn mandatory(mut self) -> QueryPart {
            self.is_mandatory = true;
            self
        }

        fn subtracted(mut self) -> QueryPart {
            self.is_subtracted = true;
            self
        }

        fn negated(mut self) -> QueryPart {
            self.is_inverted = true;
            self
        }

        fn no_expand(mut self) -> QueryPart {
            if let QueryPartType::Term = self.part_type {
                self.auto_suffix_wildcard = false;
                self
            } else {
                panic!("Tried to call no_expand test function on non-term query part");
            }
        }

        fn no_term(mut self) -> QueryPart {
            if matches!(self.part_type, QueryPartType::Term) && self.term.is_some() {
                self.term = None;
                self
            } else {
                panic!("Tried to call no_term test function on non-term query part");
            }
        }

        fn with_searched_terms(mut self, terms_searched: Vec<&str>) -> QueryPart {
            if self.term.is_some() {
                self.terms_searched = Some(
                    terms_searched.into_iter()
                        .map(|s| s.to_owned())
                        .collect()
                );
                self
            } else {
                panic!("Tried to call with_searched_terms test function on query part with no terms");
            }
        }

        fn with_original_term(mut self, original_term: &str) -> QueryPart {
            if self.term.is_some() {
                self.original_term = Some(original_term.to_owned());
                self
            } else {
                panic!("Tried to call with_searched_terms test function on query part with no terms");
            }
        }

        fn with_field(mut self, field_name: &str) -> QueryPart {
            self.field_name = Some(field_name.to_owned());
            self
        }

        fn with_suffix(mut self) -> QueryPart {
            if matches!(self.part_type, QueryPartType::Term) && self.original_term.is_some() {
                self.suffix_wildcard = true;
                self
            } else {
                panic!("Tried to call no_term test function on non-term query part");
            }
        }

        fn with_corrected(mut self) -> QueryPart {
            if matches!(self.part_type, QueryPartType::Term) {
                self.is_corrected = true;
                self
            } else {
                panic!("Tried to call no_term test function on non-term query part");
            }
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
            "8080", "title", "body", "not", "invalidfield", "我", "他"
        ] {
            term_infos.insert(
                SmartString::from(term),
                &TERM_INFO,
            );
        }

        Dictionary { term_infos }
    }

    fn wrap_in_parentheses(query_parts: Vec<QueryPart>) -> QueryPart {
        QueryPart {
            is_mandatory: false,
            is_subtracted: false,
            is_inverted: false,
            is_corrected: false,
            auto_suffix_wildcard: false,
            suffix_wildcard: false,
            is_suffixed: false,
            original_term: None,
            term: None,
            terms_searched: None,
            part_type: QueryPartType::Bracket,
            field_name: None,
            children: Some(query_parts),
            weight: 1.0,
        }
    }

    fn get_term(term: &str) -> QueryPart {
        QueryPart {
            is_mandatory: false,
            is_subtracted: false,
            is_inverted: false,
            is_corrected: false,
            auto_suffix_wildcard: true,
            suffix_wildcard: false,
            is_suffixed: false,
            original_term: Some(term.to_owned()),
            term: Some(term.to_owned()),
            terms_searched: Some(vec![term.to_owned()]),
            part_type: QueryPartType::Term,
            field_name: None,
            children: None,
            weight: 1.0,
        }
    }

    fn get_lorem() -> QueryPart {
        get_term("lorem")
    }

    fn get_ipsum() -> QueryPart {
        get_term("ipsum")
    }

    fn get_phrase(mut children: Vec<QueryPart>) -> QueryPart {
        for child in children.iter_mut() {
            child.auto_suffix_wildcard = false;
        }

        QueryPart {
            is_mandatory: false,
            is_subtracted: false,
            is_inverted: false,
            is_corrected: false,
            auto_suffix_wildcard: false,
            suffix_wildcard: false,
            is_suffixed: false,
            original_term: None,
            term: None,
            terms_searched: None,
            part_type: QueryPartType::Phrase,
            field_name: None,
            children: Some(children),
            weight: 1.0,
        }
    }

    pub fn parse(query: &str) -> Vec<QueryPart> {
        let tokenizer = ascii::new_with_options(&InfiLanguageConfig {
            lang: "ascii".to_owned(),
            options: InfiLanguageConfigOpts::default(),
        });

        super::parse_query(
            query.to_owned(),
            &tokenizer,
            &vec!["title".to_owned(), "body".to_owned(), "heading".to_owned()],
            true,
            &get_dictionary(),
        )
    }

    pub fn parse_wo_pos(query: &str) -> Vec<QueryPart> {
        let tokenizer = ascii::new_with_options(&InfiLanguageConfig {
            lang: "ascii_stemmer".to_owned(),
            options: InfiLanguageConfigOpts::default(),
        });

        super::parse_query(
            query.to_owned(),
            &tokenizer,
            &vec!["title".to_owned(), "body".to_owned(), "heading".to_owned()],
            false,
            &get_dictionary(),
        )
    }

    pub fn parse_zn(query: &str) -> Vec<QueryPart> {
        let tokenizer = chinese::new_with_options(&InfiLanguageConfig {
            lang: "chinese".to_owned(),
            options: InfiLanguageConfigOpts::default(),
        });

        super::parse_query(
            query.to_owned(),
            &tokenizer,
            &vec!["title".to_owned(), "body".to_owned(), "heading".to_owned()],
            false,
            &get_dictionary(),
        )
    }

    // The tokenizer will remove stop words if they are not even indexed
    pub fn parse_with_sw_removal(query: &str) -> Vec<QueryPart> {
        let tokenizer = ascii::new_with_options(&InfiLanguageConfig {
            lang: "ascii".to_owned(),
            options: InfiLanguageConfigOpts {
                stop_words: None,
                ignore_stop_words: Some(true),
                stemmer: None,
                max_term_len: None,
            },
        });

        super::parse_query(
            query.to_owned(),
            &tokenizer,
            &vec!["title".to_owned(), "body".to_owned(), "heading".to_owned()],
            true,
            &get_dictionary(),
        )
    }

    #[test]
    fn free_text_test() {
        assert_eq!(parse(""), vec![]);
        assert_eq!(parse(" "), vec![]);
        assert_eq!(parse("lorem ipsum"), vec![get_lorem(), get_ipsum()]);
        assert_eq!(parse("lorem ipsum "), vec![get_lorem().no_expand(), get_ipsum().no_expand()]);
        assert_eq!(parse_with_sw_removal("for by lorem and"), vec![
            get_term("for").no_term(), get_term("by").no_term(),
            get_lorem(), get_term("and").no_term(),
        ]);
    }

    #[test]
    fn wildcard_suffix_test() {
        assert_eq!(parse("lorem* ipsum"), vec![get_lorem().with_suffix(), get_ipsum()]);
        assert_eq!(parse("lorem* ipsum "), vec![
            get_lorem().no_expand().with_suffix(),
            get_ipsum().no_expand(),
        ]);
        assert_eq!(parse("lorem ipsum*"), vec![
            get_lorem(),
            get_ipsum().with_suffix(),
        ]);
        assert_eq!(parse("lorem* ipsum* "), vec![
            get_lorem().no_expand().with_suffix(),
            get_ipsum().no_expand().with_suffix(),
        ]);
        assert_eq!(parse_with_sw_removal("for* by* lorem and"), vec![
            get_term("for").no_term().with_suffix(),
            get_term("by").no_term().with_suffix(),
            get_lorem(), get_term("and").no_term(),
        ]);
    }

    #[test]
    fn boolean_test() {
        assert_eq!(parse("-"), vec![]);
        assert_eq!(parse(" -"), vec![]);
        assert_eq!(parse("+"), vec![]);
        assert_eq!(parse(" +"), vec![]);
        assert_eq!(parse("~"), vec![]);
        assert_eq!(parse(" ~"), vec![]);
        assert_eq!(parse("-lorem"), vec![get_lorem().subtracted()]);
        assert_eq!(parse(" -lorem"), vec![get_lorem().subtracted()]);
        assert_eq!(parse("+lorem"), vec![get_lorem().mandatory()]);
        assert_eq!(parse(" +lorem"), vec![get_lorem().mandatory()]);
        assert_eq!(parse("~lorem"), vec![get_lorem().negated()]);
        assert_eq!(parse(" ~lorem"), vec![get_lorem().negated()]);
        assert_eq!(parse("--lorem"), vec![get_lorem().subtracted()]);
        assert_eq!(parse(" --lorem"), vec![get_lorem().subtracted()]);
        assert_eq!(parse("++lorem"), vec![get_lorem().mandatory()]);
        assert_eq!(parse(" ++lorem"), vec![get_lorem().mandatory()]);
        assert_eq!(parse("~~lorem"), vec![get_lorem().negated()]);
        assert_eq!(parse(" ~~lorem"), vec![get_lorem().negated()]);

        // Whitespace sensitivity after
        assert_eq!(parse("- lorem"), vec![get_lorem()]);
        assert_eq!(parse(" - lorem"), vec![get_lorem()]);
        assert_eq!(parse("+ lorem"), vec![get_lorem()]);
        assert_eq!(parse(" + lorem"), vec![get_lorem()]);
        assert_eq!(parse("~ lorem"), vec![get_lorem()]);
        assert_eq!(parse(" ~ lorem"), vec![get_lorem()]);

        assert_eq!(parse(" +lorem ipsum"), vec![get_lorem().mandatory(), get_ipsum()]);
        assert_eq!(parse(" lorem +ipsum"), vec![get_lorem(), get_ipsum().mandatory()]);
        assert_eq!(parse("-lorem ipsum"), vec![get_lorem().subtracted(), get_ipsum()]);
        assert_eq!(parse("lorem -ipsum"), vec![get_lorem(), get_ipsum().subtracted()]);
        assert_eq!(parse(" ~lorem ipsum"), vec![get_lorem().negated(), get_ipsum()]);
        assert_eq!(parse(" lorem ~ipsum"), vec![get_lorem(), get_ipsum().negated()]);

        assert_eq!(parse("lorem-ipsum"), vec![get_lorem(), get_ipsum()]);
        assert_eq!(parse("lorem+ipsum"), vec![get_lorem(), get_ipsum()]);
        assert_eq!(parse("lorem~ipsum"), vec![get_lorem(), get_ipsum()]);
        assert_eq!(parse("lorem -ipsum"), vec![get_lorem(), get_ipsum().subtracted()]);
        assert_eq!(parse("lorem +ipsum"), vec![get_lorem(), get_ipsum().mandatory()]);
        assert_eq!(parse("lorem ~ipsum"), vec![get_lorem(), get_ipsum().negated()]);

        assert_eq!(parse("\"lorem\"-ipsum"), vec![get_phrase(vec![get_lorem().mandatory()]), get_ipsum().subtracted()]);
        assert_eq!(parse("(lorem)-ipsum"), vec![wrap_in_parentheses(vec![get_lorem()]), get_ipsum().subtracted()]);
        assert_eq!(parse("\"lorem\" -ipsum"), vec![get_phrase(vec![get_lorem().mandatory()]), get_ipsum().subtracted()]);
        assert_eq!(parse("(lorem) -ipsum"), vec![wrap_in_parentheses(vec![get_lorem()]), get_ipsum().subtracted()]);
        assert_eq!(parse("ipsum-\"lorem\""), vec![get_ipsum(), get_phrase(vec![get_lorem().mandatory()])]);
        assert_eq!(parse("ipsum-(lorem)"), vec![get_ipsum(), wrap_in_parentheses(vec![get_lorem()])]);
        assert_eq!(parse("ipsum -\"lorem\""), vec![get_ipsum(), get_phrase(vec![get_lorem().mandatory()]).subtracted()]);
        assert_eq!(parse("ipsum -(lorem)"), vec![get_ipsum(), wrap_in_parentheses(vec![get_lorem()]).subtracted()]);

        assert_eq!(parse(" +lorem +ipsum +lorem"), vec![
            get_lorem().mandatory(),
            get_ipsum().mandatory(),
            get_lorem().mandatory(),
        ]);
        assert_eq!(parse(" -lorem -ipsum -lorem"), vec![
            get_lorem().subtracted(),
            get_ipsum().subtracted(),
            get_lorem().subtracted(),
        ]);

        assert_eq!(parse(" +~lorem -ipsum -~lorem"), vec![
            get_lorem().negated().mandatory(),
            get_ipsum().subtracted(),
            get_lorem().negated().subtracted(),
        ]);

        assert_eq!(
            parse("+lorem +lorem -ipsum"),
            vec![
                get_lorem().mandatory(),
                get_lorem().mandatory(),
                get_ipsum().subtracted(),
            ]
        );

        assert_eq!(parse_with_sw_removal("~for +by"), vec![
            get_term("for").no_term().negated(), get_term("by").no_term().mandatory(),
        ]);
        assert_eq!(parse_with_sw_removal("-for +lorem"), vec![
            get_term("for").no_term().subtracted(), get_lorem().mandatory(),
        ]);
    }

    #[test]
    fn phrase_test() {
        assert_eq!(parse_wo_pos("\"lorem ipsum\""), vec![get_term("lorem"), get_term("ipsum")]);

        assert_eq!(parse("\"lorem ipsum\""), vec![
            get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()]),
        ]);
        assert_eq!(
            parse("\"(lorem ipsum)\""),
            vec![get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()])],
        );
        assert_eq!(
            parse("lorem\"lorem ipsum\""),
            vec![get_lorem(), get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()])],
        );
        assert_eq!(
            parse("\"lorem ipsum\"lorem\"lorem ipsum\""),
            vec![
                get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()]),
                get_lorem(),
                get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()]),
            ]
        );
        assert_eq!(
            parse("\"lorem ipsum\" lorem \"lorem ipsum\""),
            vec![
                get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()]),
                get_lorem().no_expand(),
                get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()]),
            ]
        );
        assert_eq!(
            parse("\"lore ipsum\""),
            vec![
                get_phrase(vec![
                    get_lorem().mandatory()
                        .with_corrected()
                        .with_original_term("lore")
                        .with_searched_terms(vec!["lore", "lorem"]),
                    get_ipsum().mandatory(),
                ]),
            ],
        );
        assert_eq!(
            parse("\"nonexistentterm lore ipsum\""),
            vec![
                get_phrase(vec![
                    get_term("nonexistentterm").no_term(),
                    get_lorem().mandatory()
                        .with_corrected()
                        .with_original_term("lore")
                        .with_searched_terms(vec!["lore", "lorem"]),
                    get_ipsum().mandatory(),
                ]),
            ]
        );
        assert_eq!(
            parse_with_sw_removal("\"for by lorem and\""),
            vec![
                get_phrase(vec![
                    get_term("for").no_term(),
                    get_term("by").no_term(),
                    get_lorem().mandatory(),
                    get_term("and").no_term(),
                ]),
            ]
        );
        assert_eq!(
            parse_with_sw_removal("\"l'orem for by ipsum and\""),
            vec![
                get_phrase(vec![
                    get_lorem().mandatory().with_searched_terms(vec!["l'orem", "l’orem", "lorem"]),
                    get_term("for").no_term(),
                    get_term("by").no_term(),
                    get_ipsum().mandatory(),
                    get_term("and").no_term(),
                ]),
            ]
        );
        assert_eq!(
            parse_with_sw_removal("\"lorem ipsum\""),
            vec![
                get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()]),
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
            parse("title:+lorem +ipsum"),
            vec![get_lorem().with_field("title").mandatory(), get_ipsum().mandatory()]
        );
        assert_eq!(
            parse("title:(+lorem +ipsum)"),
            vec![
                wrap_in_parentheses(
                    vec![get_lorem().mandatory(), get_ipsum().mandatory()],
                ).with_field("title"),
            ]
        );
        assert_eq!(
            parse("title:~ lorem ipsum)"),
            vec![get_lorem().with_field("title"), get_ipsum()]
        );
        assert_eq!(
            parse("title:-lorem ipsum)"),
            vec![
                get_lorem().with_field("title").subtracted(),
                get_ipsum(),
            ]
        );
        assert_eq!(
            parse("title:~lorem ipsum)"),
            vec![get_lorem().negated().with_field("title"), get_ipsum()]
        );
        assert_eq!(
            parse("title: lorem ~ipsum)"),
            vec![get_lorem().with_field("title"), get_ipsum().negated()]
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
            parse("invalidfield: lorem ~ipsum)"),
            vec![
                get_term("invalidfield"),
                get_lorem(),
                get_ipsum().negated()
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
            parse("http://localhost:8080 +lorem"),
            vec![
                get_term("http"),
                get_term("localhost"),
                get_term("8080"),
                get_lorem().mandatory(),
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
    fn spelling_correction_test() {
        assert_eq!(parse("lore"), vec![
            get_lorem()
                .with_corrected()
                .with_original_term("lore")
                .with_searched_terms(vec!["lore", "lorem"]),
        ]);
        assert_eq!(parse("+lore +ipsum"), vec![
            get_lorem()
                .with_corrected()
                .with_original_term("lore")
                .with_searched_terms(vec!["lore", "lorem"])
                .mandatory(),
            get_ipsum().mandatory(),
        ]);
        assert_eq!(parse("+\"lore ipsum\" +ipsum"), vec![
            get_phrase(vec![
                get_lorem()
                    .mandatory()
                    .with_corrected()
                    .with_original_term("lore")
                    .with_searched_terms(vec!["lore", "lorem"]),
                get_ipsum().mandatory(),
            ]).mandatory(),
            get_ipsum().mandatory(),
        ]);
    }

    #[test]
    fn misc_test() {
        assert_eq!(
            parse("title: \"lorem ipsum\""),
            vec![
                get_phrase(vec![
                    get_lorem().mandatory(), get_ipsum().mandatory(),
                ]).with_field("title")
            ]
        );

        assert_eq!(
            parse("title:+(+lorem +ipsum) -(lorem ipsum) body:(lorem ~ipsum)"),
            vec![
                wrap_in_parentheses(vec![get_lorem().mandatory(), get_ipsum().mandatory()])
                    .mandatory()
                    .with_field("title"),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum(),]).subtracted(),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum().negated()])
                    .with_field("body")
            ]
        );

        assert_eq!(
            parse("title:(+lorem +ipsum) title:~(lorem ipsum) body:(lorem ~ipsum)"),
            vec![
                wrap_in_parentheses(vec![get_lorem().mandatory(), get_ipsum().mandatory()]).with_field("title"),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum(),]).negated().with_field("title"),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum().negated()])
                    .with_field("body")
            ]
        );

        assert_eq!(
            parse("+body:ipsum +http://localhost:8080 -(title:lorem)"),
            vec![
                get_ipsum().with_field("body").mandatory(),
                get_term("http").mandatory(),
                get_term("localhost"),
                get_term("8080"),
                wrap_in_parentheses(vec![get_lorem().with_field("title")]).subtracted(),
            ]
        );

        assert_eq!(
            parse("title:\"+lorem +ipsum\" ~title:(\"lorem ipsum\") body:(lorem ~ipsum)"),
            vec![
                get_phrase(
                    vec![get_lorem().mandatory(), get_ipsum().mandatory()],
                ).with_field("title"),
                wrap_in_parentheses(
                    vec![get_phrase(vec![get_lorem().mandatory(), get_ipsum().mandatory()])],
                ).with_field("title").negated(),
                wrap_in_parentheses(
                    vec![get_lorem(), get_ipsum().negated()],
                ).with_field("body")
            ]
        );

        assert_eq!(
            parse("title:(+lorem +body:(lorem ipsum)) -title:((body:\"lorem\") ipsum) body:(lorem ~ipsum)"),
            vec![
                wrap_in_parentheses(vec![
                    get_lorem().mandatory(),
                    wrap_in_parentheses(vec![
                        get_lorem(),
                        get_ipsum(),
                    ]).with_field("body").mandatory(),
                ]).with_field("title"),
                wrap_in_parentheses(vec![
                    wrap_in_parentheses(vec![
                        get_phrase(vec![get_lorem().mandatory()]).with_field("body"),
                    ]),
                    get_ipsum(),
                ]).with_field("title").subtracted(),
                wrap_in_parentheses(vec![get_lorem(), get_ipsum().negated()]).with_field("body")
            ]
        );

        assert_eq!(
            parse("title:+lorem +ipsum -lorem ipsum body:lorem ~ipsum"),
            vec![
                get_lorem().no_expand().with_field("title").mandatory(),
                get_ipsum().no_expand().mandatory(),
                get_lorem().no_expand().subtracted(),
                get_ipsum().no_expand(),
                get_lorem().with_field("body"),
                get_ipsum().negated(),
            ]
        );

        assert_eq!(
            parse("(~lorem ipsum)"),
            vec![
                wrap_in_parentheses(vec![
                    get_lorem().negated(),
                    get_ipsum(),
                ]),
            ]
        );

        assert_eq!(
            parse("title:(lorem) \\(-lorem ipsum) \\+title\\:lorem\\+ipsum \\(lorem) \\-lorem ipsum body\\:lorem \\~ipsum"),
            vec![
                wrap_in_parentheses(vec![get_term("lorem")]).with_field("title"),
                get_term("lorem").subtracted(),
                get_term("ipsum"),
                get_term("title"),
                get_term("lorem"),
                get_term("ipsum"),
                get_term("lorem"),
                get_term("lorem"),
                get_term("ipsum"),
                get_term("body"),
                get_term("lorem"),
                get_term("ipsum"),
            ]
        );

        assert_eq!(
            parse("((lorem +ipsum) lorem) (lorem()~ipsum)"),
            vec![
                wrap_in_parentheses(vec![wrap_in_parentheses(vec![get_lorem(), get_ipsum().mandatory()]), get_lorem(),]),
                wrap_in_parentheses(vec![get_lorem(), wrap_in_parentheses(vec![]), get_ipsum().negated()]),
            ]
        );
    }

    #[test]
    fn zn_test() {
        assert_eq!(parse("我-(lorem)"), vec![wrap_in_parentheses(vec![get_lorem()])]);
        assert_eq!(parse_zn("我-(lorem)"), vec![get_term("我"), wrap_in_parentheses(vec![get_lorem()]).subtracted()]);
    }
}
