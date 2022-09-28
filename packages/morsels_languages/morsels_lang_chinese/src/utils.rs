
/*
 No fancy jieba-rs tokenization, etc.
 1. The bundle size blows up
 2. Document recall is severely impacted when using 精确模式
 3. 全模式 is too difficult to deal for now
 4. Query term proximity ranking should compensate for precision.
*/

use std::borrow::Cow;

use morsels_lang_ascii::utils::{intra_filter, separating_filter};


pub fn split_terms(c: char) -> bool {
    c.is_whitespace() || separating_filter(c) || is_supporting_chinese_char(c)
}


// Adapted from https://github.com/alsotang/is_chinese_rs/blob/main/src/lib.rs
pub fn is_chinese_char(c: char) -> bool {
    match c as u32 {
        0x4e00..=0x9fff => true,
        0x3400..=0x4dbf => true,   // CJK Unified Ideographs Extension A
        0x20000..=0x2a6df => true, // CJK Unified Ideographs Extension B
        0x2a700..=0x2b73f => true, // CJK Unified Ideographs Extension C
        0x2b740..=0x2b81f => true, // CJK Unified Ideographs Extension D
        0x2b820..=0x2ceaf => true, // CJK Unified Ideographs Extension E
        0x3300..=0x33ff => true,   // https://en.wikipedia.org/wiki/CJK_Compatibility
        0xfe30..=0xfe4f => true,   // https://en.wikipedia.org/wiki/CJK_Compatibility_Forms
        0xf900..=0xfaff => true,   // https://en.wikipedia.org/wiki/CJK_Compatibility_Ideographs
        0x2f800..=0x2fa1f => true, // https://en.wikipedia.org/wiki/CJK_Compatibility_Ideographs_Supplement
        _ => false,
    }
}

fn is_supporting_chinese_char(c: char) -> bool {
    match c as u32 {
        0x00b7 |            //·
        0x00d7 |            //×
        0x2026 |            //…
        0x3001 |            //、
        0x300a |            //《
        0x300b |            //》
        0x300e |            //『
        0x300f |            //』
        0x3010 |            //【
        0x3011 => true,     //】
        _ => false,
    }
}


pub fn term_filter(input: Cow<str>) -> Cow<str> {
    let mut char_iter = input.char_indices()
        .filter(|(_idx, c)| split_terms(*c) || intra_filter(*c));

    if let Some((char_start, c)) = char_iter.next() {
        let mut output: Vec<u8> = Vec::with_capacity(input.len());
        output.extend_from_slice(input[0..char_start].as_bytes());
        let mut prev_char_end = char_start + c.len_utf8();

        for (char_start, c) in char_iter {
            output.extend_from_slice(input[prev_char_end..char_start].as_bytes());
            prev_char_end = char_start + c.len_utf8();
        }
        output.extend_from_slice(input[prev_char_end..].as_bytes());

        Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
    } else {
        input
    }
}
