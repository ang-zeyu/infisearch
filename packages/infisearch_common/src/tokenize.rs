use std::borrow::Cow;

use crate::dictionary::Dictionary;


// When None is yielded, it indicates a positional gap
pub type TermIter<'a> = Box<dyn Iterator<Item = Option<Cow<'a, str>>> + 'a>;

pub trait IndexerTokenizer {
    fn tokenize<'a>(&'a self, text: &'a mut str) -> TermIter<'a>;
}

pub trait SearchTokenizer {
    fn search_tokenize(
        &self,
        query_chars: &[char],
        query_chars_offset: usize,
        query_chars_offset_end: usize,
        escape_indices: &[usize],
        dict: &Dictionary,
    ) -> SearchTokenizeResult;

    fn is_stop_word(&self, term: &str) -> bool;

    fn is_valid_prefix_op_terminator(&self, c: char) -> bool;
}

pub struct SearchTokenizeResult {
    pub auto_suffix_wildcard: bool,
    pub terms: Vec<SearchTokenizeTerm>,
}

pub struct SearchTokenizeTerm {
    pub term: Option<String>,
    pub term_inflections: Vec<String>,
    pub original_term: String,
    pub suffix_wildcard: bool,
    pub is_corrected: bool,
    pub prefix_ops: PrefixResult,
}

#[derive(Default)]
pub struct PrefixResult {
    pub is_mandatory: bool,
    pub is_subtracted: bool,
    pub is_inverted: bool,
}

#[inline(never)]
pub fn get_prefix_ops(
    idx: usize,
    offset: usize,
    start_limit: usize,
    query_chars: &[char],
    escape_indices: &[usize],
    tokenizer: &dyn SearchTokenizer,
) -> PrefixResult {
    let mut res = PrefixResult::default();

    if idx >= offset  {
        let idx = idx - offset;

        if idx >= start_limit {
            let escape_limit = escape_indices
                .iter()
                .rev()
                .find(|&&escape_idx| escape_idx < idx)
                .map(|&first_escape_idx| idx - (first_escape_idx + 1))
                .unwrap_or(idx + 1);
            let start_limit = (idx - start_limit) + 1;
    
            let limit = start_limit.min(escape_limit);

            /*
             Possible ops:
             +~
             ~+ will be interpreted as +~
             -~
             ~- will be interpreted as -~
    
             +- mutually exclusive. Use the one that appears first.
            */
    
            for &c in unsafe { query_chars.get_unchecked(..idx + 1) }.iter().rev().take(limit) {
                match c {
                    '+' => {
                        if !res.is_mandatory && !res.is_subtracted {
                            res.is_mandatory = true;
                        }
                    },
                    '-' => {
                        if !res.is_mandatory && !res.is_subtracted {
                            res.is_subtracted = true;
                        }
                    },
                    '~' => {
                        res.is_inverted = true;
                    },
                    c => {
                        if !tokenizer.is_valid_prefix_op_terminator(c) {
                            // Operators must be preceded by a whitespace
                            return PrefixResult::default();
                        }
                        break;
                    },
                }
            }
        }
    }

    res
}
