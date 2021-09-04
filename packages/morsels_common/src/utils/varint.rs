static VALUE_MASK: u8 = 0x7f; // 0111 1111
static CONTINUATION_MASK: u8 = 0x80; // 1000 0000

pub fn decode_var_int(slice: &[u8]) -> (u32, usize) {
    let mut current_value: u32 = 0;
    let mut shift_amount: u8 = 0;
    let mut pos = 0;

    while pos < slice.len() {
        let current_byte = slice[pos];
        let mask_result = VALUE_MASK & current_byte;
        current_value |= (mask_result as u32) << shift_amount;

        pos += 1;
        if (CONTINUATION_MASK & current_byte) != 0 {
            break;
        }
        shift_amount += 7;
    }

    (current_value, pos)
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::{decode_var_int, CONTINUATION_MASK};

    #[test]
    fn test_decode() {
        assert_eq!(
            decode_var_int(&[]),
            (0, 0)
        );

        assert_eq!(
            decode_var_int(&[CONTINUATION_MASK | 0]),
            (0, 1)
        );

        assert_eq!(
            decode_var_int(&[CONTINUATION_MASK | 64]),
            (64, 1)
        );

        assert_eq!(
            decode_var_int(&[CONTINUATION_MASK | 127]),
            (127, 1)
        );

        assert_eq!(
            decode_var_int(&[0, CONTINUATION_MASK | 127]),
            (16256, 2)
        );

        assert_eq!(
            decode_var_int(&[10, CONTINUATION_MASK | 127]),
            (16266, 2)
        );

        assert_eq!(
            decode_var_int(&[127, CONTINUATION_MASK | 127]),
            (16383, 2)
        );

        assert_eq!(
            decode_var_int(&[127, 127, 127, 127, CONTINUATION_MASK | 15]),
            (u32::MAX, 5)
        );
    }
}
