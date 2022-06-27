use std::borrow::Cow;

#[inline(always)]
pub fn split_terms(c: char) -> bool {
    c.is_whitespace() || separating_filter(c)
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

// Things that commonly "separate" words, apart from whitespaces
pub fn separating_filter(c: char) -> bool {
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
        '。' |
        '《' |
        '》' |
        '…' |
        '•' |
        '?' |
        '!' |
        ',' |
        '.' |
        '@' |
        '\u{2045}' | // ⁅  [LEFT SQUARE BRACKET WITH QUILL]
        '\u{2772}' | // ❲  [LIGHT LEFT TORTOISE SHELL BRACKET ORNAMENT]
        '\u{FF3B}' | // ［  [FULLWIDTH LEFT SQUARE BRACKET]
        '\u{2046}' | // ⁆  [RIGHT SQUARE BRACKET WITH QUILL]
        '\u{2773}' | // ❳  [LIGHT RIGHT TORTOISE SHELL BRACKET ORNAMENT]
        '\u{FF3D}' | // ］  [FULLWIDTH RIGHT SQUARE BRACKET]
        '\u{FF3C}' | // ＼  [FULLWIDTH REVERSE SOLIDUS]
        '\u{207D}' | // ⁽  [SUPERSCRIPT LEFT PARENTHESIS]
        '\u{208D}' | // ₍  [SUBSCRIPT LEFT PARENTHESIS]
        '\u{2768}' | // ❨  [MEDIUM LEFT PARENTHESIS ORNAMENT]
        '\u{276A}' | // ❪  [MEDIUM FLATTENED LEFT PARENTHESIS ORNAMENT]
        '\u{FF08}' | // （  [FULLWIDTH LEFT PARENTHESIS]
        '\u{2E28}' | // ⸨  [LEFT DOUBLE PARENTHESIS]
        '\u{207E}' | // ⁾  [SUPERSCRIPT RIGHT PARENTHESIS]
        '\u{208E}' | // ₎  [SUBSCRIPT RIGHT PARENTHESIS]
        '\u{2769}' | // ❩  [MEDIUM RIGHT PARENTHESIS ORNAMENT]
        '\u{276B}' | // ❫  [MEDIUM FLATTENED RIGHT PARENTHESIS ORNAMENT]
        '\u{FF09}' | // ）  [FULLWIDTH RIGHT PARENTHESIS]
        '\u{2E29}' | // ⸩  [RIGHT DOUBLE PARENTHESIS]
        '\u{2774}' | // ❴  [MEDIUM LEFT CURLY BRACKET ORNAMENT]
        '\u{FF5B}' | // ｛  [FULLWIDTH LEFT CURLY BRACKET]
        '\u{2775}' | // ❵  [MEDIUM RIGHT CURLY BRACKET ORNAMENT]
        '\u{FF5D}' | // ｝  [FULLWIDTH RIGHT CURLY BRACKET]
        '\u{FF06}' | // ＆  [FULLWIDTH AMPERSAND]
        '\u{00AB}' | // «  [LEFT-POINTING DOUBLE ANGLE QUOTATION MARK]
        '\u{00BB}' | // »  [RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK]
        '\u{201C}' | // “  [LEFT DOUBLE QUOTATION MARK]
        '\u{201D}' | // ”  [RIGHT DOUBLE QUOTATION MARK]
        '\u{201E}' | // „  [DOUBLE LOW-9 QUOTATION MARK]
        '\u{275D}' | // ❝  [HEAVY DOUBLE TURNED COMMA QUOTATION MARK ORNAMENT]
        '\u{275E}' | // ❞  [HEAVY DOUBLE COMMA QUOTATION MARK ORNAMENT]
        '\u{276E}' | // ❮  [HEAVY LEFT-POINTING ANGLE QUOTATION MARK ORNAMENT]
        '\u{276F}' | // ❯  [HEAVY RIGHT-POINTING ANGLE QUOTATION MARK ORNAMENT]
        '\u{FF02}' | // ＂  [FULLWIDTH QUOTATION MARK]
        '\u{276C}' | // ❬  [MEDIUM LEFT-POINTING ANGLE BRACKET ORNAMENT]
        '\u{2770}' | // ❰  [HEAVY LEFT-POINTING ANGLE BRACKET ORNAMENT]
        '\u{FF1C}' | // ＜  [FULLWIDTH LESS-THAN SIGN]
        '\u{276D}' | // ❭  [MEDIUM RIGHT-POINTING ANGLE BRACKET ORNAMENT]
        '\u{2771}' | // ❱  [HEAVY RIGHT-POINTING ANGLE BRACKET ORNAMENT]
        '\u{FF1E}' | // ＞  [FULLWIDTH GREATER-THAN SIGN]
        '\u{FF03}' | // ＃  [FULLWIDTH NUMBER SIGN]
        '\u{FF1A}' | // ：  [FULLWIDTH COLON]
        '\u{204F}' | // ⁏  [REVERSED SEMICOLON]
        '\u{FF1B}' | // ；  [FULLWIDTH SEMICOLON]
        '\u{2053}' | // ⁓  [SWUNG DASH]
        '\u{FF5E}' | // ～  [FULLWIDTH TILDE]
        '\u{2038}' | // ‸  [CARET]
        '\u{FF3E}' | // ＾  [FULLWIDTH CIRCUMFLEX ACCENT]
        '\u{207C}' | // ⁼  [SUPERSCRIPT EQUALS SIGN]
        '\u{208C}' | // ₌  [SUBSCRIPT EQUALS SIGN]
        '\u{FF1D}' | // ＝  [FULLWIDTH EQUALS SIGN]
        '\u{207A}' | // ⁺  [SUPERSCRIPT PLUS SIGN]
        '\u{208A}' | // ₊  [SUBSCRIPT PLUS SIGN]
        '\u{FF0B}' | // ＋  [FULLWIDTH PLUS SIGN]
        '\u{204E}' | // ⁎  [LOW ASTERISK]
        '\u{FF0A}' | // ＊  [FULLWIDTH ASTERISK]
        '\u{2044}' | // ⁄  [FRACTION SLASH]
        '\u{FF0F}' | // ／  [FULLWIDTH SOLIDUS]
        '\u{2049}' | // ⁉  [EXCLAMATION QUESTION MARK]
        '\u{FF1F}' | // ？  [FULLWIDTH QUESTION MARK]
        '\u{2047}' | // ⁇  [DOUBLE QUESTION MARK]
        '\u{FF01}' | // ！  [FULLWIDTH EXCLAMATION MARK]
        '\u{203C}' | // ‼  [DOUBLE EXCLAMATION MARK]
        '\u{2048}' | // ⁈  [QUESTION EXCLAMATION MARK]
        '\u{FF0C}' | // ，  [FULLWIDTH COMMA]
        '\u{FF0E}' | // ．  [FULLWIDTH FULL STOP]
        '\u{FF20}' | // ＠  [FULLWIDTH COMMERCIAL AT]
        // More controversial ones
        '\u{2013}' | // –  [EN DASH]
        '\u{2014}' | // —  [EM DASH]
        '\u{2018}' | // ‘  [LEFT SINGLE QUOTATION MARK]
        '\u{2019}' | // ’  [RIGHT SINGLE QUOTATION MARK]
        '\u{201A}' | // ‚  [SINGLE LOW-9 QUOTATION MARK]
        '\u{201B}' | // ‛  [SINGLE HIGH-REVERSED-9 QUOTATION MARK]
        '\u{2039}' | // ‹  [SINGLE LEFT-POINTING ANGLE QUOTATION MARK]
        '\u{203A}' | // ›  [SINGLE RIGHT-POINTING ANGLE QUOTATION MARK]
        '\u{275B}' | // ❛  [HEAVY SINGLE TURNED COMMA QUOTATION MARK ORNAMENT]
        '\u{275C}'   // ❜  [HEAVY SINGLE COMMA QUOTATION MARK ORNAMENT]
        => true,
        _ => false
    }
}

// Things that commonly appear within words
pub fn intra_filter(c: char) -> bool {
    match c {
        '\'' |
        '-' |
        '_' |
        '\u{2010}' | // ‐  [HYPHEN]
        '\u{2011}' | // ‑  [NON-BREAKING HYPHEN]
        '\u{2012}' | // ‒  [FIGURE DASH]
        '\u{207B}' | // ⁻  [SUPERSCRIPT MINUS]
        '\u{208B}' | // ₋  [SUBSCRIPT MINUS]
        '\u{FF0D}' | // －  [FULLWIDTH HYPHEN-MINUS]
        '\u{FF3F}' | // ＿  [FULLWIDTH LOW LINE]
        '\u{FF07}' | // ＇  [FULLWIDTH APOSTROPHE]
        '\u{2032}' | // ′  [PRIME]
        '\u{2035}' | // ‵  [REVERSED PRIME]
        // Moved from above
        '\u{2033}' | // ″  [DOUBLE PRIME]
        '\u{2036}'   // ‶  [REVERSED DOUBLE PRIME]
        => true,
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
                do_delete = intra_filter(c);
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
        assert("a1a-a2a", "a1aa2a");
        assert("a1a-'_a2a", "a1aa2a");
        assert("a1a⥄a2a", "a1a⥄a2a");
        assert("a1a-'⥄a2a", "a1a⥄a2a");
    }

    #[test]
    fn removes_starting_characters() {
        assert("-a1aa2a", "a1aa2a");
        assert("⥄a1aa2a", "a1aa2a");
        assert("⥄⥄a1aa2a", "a1aa2a");
    }

    #[test]
    fn removes_ending_characters() {
        assert("a1aa2a-", "a1aa2a");
        assert("a1aa2a⥄", "a1aa2a");
        assert("a1aa2a⥄⥄", "a1aa2a");
    }

    #[test]
    fn removes_all_characters() {
        assert("-a1a-a2a-", "a1aa2a");
        assert("-a1a⥄a2a-", "a1a⥄a2a");
    }
}
