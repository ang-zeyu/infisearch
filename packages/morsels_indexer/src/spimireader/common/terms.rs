use std::io::Write;

#[inline(always)]
pub fn get_common_unicode_prefix_byte_len(str1: &str, str2: &str) -> usize {
    let mut byte_len = 0;
    let mut str1_it = str1.chars();
    let mut str2_it = str2.chars();

    loop {
        let str1_next = str1_it.next();
        let str2_next = str2_it.next();
        if str1_next == None || str2_next == None || (str1_next.unwrap() != str2_next.unwrap()) {
            break;
        }

        byte_len += str1_next.unwrap().len_utf8();
    }

    byte_len
}

#[inline(always)]
pub fn frontcode_and_store_term(prev_term: &str, curr_term: &str, dict_string_writer: &mut Vec<u8>) -> (u8, u8) {
    let unicode_prefix_byte_len = get_common_unicode_prefix_byte_len(prev_term, curr_term);

    dict_string_writer.write_all(&curr_term.as_bytes()[unicode_prefix_byte_len..]).unwrap();

    (
        unicode_prefix_byte_len as u8,                     // Prefix length
        (curr_term.len() - unicode_prefix_byte_len) as u8, // Remaining length
    )
}
