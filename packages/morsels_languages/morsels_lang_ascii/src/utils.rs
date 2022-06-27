use std::borrow::Cow;

#[inline(always)]
pub fn split_terms(c: char) -> bool {
    c.is_whitespace() || intra_filter(c, false)
}

#[allow(clippy::match_like_matches_macro)]
fn boundary_filter(c: char) -> bool {
    match c {
        'a'..='z' |
        '0'..='9'
        => false,
        _ => true
    }
}

pub fn intra_filter(c: char, remove_dashes: bool) -> bool {
    match c {
        '[' |
        ']' |
        '\\' |
        '(' |
        ')' |
        '{' |
        '}' |
        '&' |
        '|' |
        '"' |
        '`' |
        '<' |
        '>' |
        '#' |
        ':' |
        ';' |
        '~' |
        '^' |
        '=' |
        '+' |
        '*' |
        '/' |
        '‘' |
        '“' |
        '”' |
        '，' |
        '。' |
        '《' |
        '》' |
        '…' |
        '—' | // emdash
        '•' |
        '?' |
        '!' |
        ',' |
        '.'
        => true,
        // May appear inside a word
        '\'' |
        '-' |
        '_' |
        '’' |
        '‑' |
        '‐'
        => remove_dashes,
        _ => false
    }
}

pub fn term_filter(input: Cow<str>) -> Cow<str> {
    let mut char_iter = input.char_indices().filter(|(_idx, c)| boundary_filter(*c));

    if let Some((mut char_start, mut c)) = char_iter.next() {
        let mut output: Vec<u8> = Vec::with_capacity(input.len());
        let mut at_start = true;
        let mut prev_char_end = 0;

        loop {
            let mut do_delete = true;
            if !(at_start && prev_char_end == char_start) {
                at_start = false;
                do_delete = intra_filter(c, true);
            }

            if do_delete {
                output.extend_from_slice(input[prev_char_end..char_start].as_bytes());
                prev_char_end = char_start + c.len_utf8();
            }

            if let Some((next_idx, next_c)) = char_iter.next() {
                char_start = next_idx;
                c = next_c;
            } else {
                output.extend_from_slice(input[prev_char_end..].as_bytes());
                let mut output = unsafe { String::from_utf8_unchecked(output) };
                if let Some((idx, c)) = output.char_indices().rev().find(|(_, c)| !boundary_filter(*c)) {
                    output.drain((idx + c.len_utf8())..);
                    return Cow::Owned(output);
                } else {
                    return Cow::Owned("".to_owned());
                }
            }
        }
    } else {
        input
    }
}

#[cfg(test)]
pub mod test {
    use std::borrow::Cow;

    use super::term_filter;

    fn assert(input: &str, expected: &str) {
        assert_eq!(term_filter(Cow::Borrowed(input)), expected);
    }

    #[test]
    fn removes_intermediate_characters() {
        assert("a1a*a2a", "a1aa2a");
        assert("a1a*!)a2a", "a1aa2a");
        assert("a1a⥄a2a", "a1a⥄a2a");
        assert("a1a*!⥄a2a", "a1a⥄a2a");
    }

    #[test]
    fn removes_starting_characters() {
        assert("*a1aa2a", "a1aa2a");
        assert("⥄a1aa2a", "a1aa2a");
        assert("⥄⥄a1aa2a", "a1aa2a");
    }

    #[test]
    fn removes_ending_characters() {
        assert("a1aa2a*", "a1aa2a");
        assert("a1aa2a⥄", "a1aa2a");
        assert("a1aa2a⥄⥄", "a1aa2a");
    }

    #[test]
    fn removes_all_characters() {
        assert("*a1a*a2a*", "a1aa2a");
        assert("*a1a⥄a2a*", "a1a⥄a2a");
    }
}
