static VALUE_MASK: u8 = 0x7f; // 0111 1111
static CONTINUATION_MASK: u8 = 0x80; // 1000 0000

#[inline(always)]
pub fn decode_var_int(slice: &[u8], pos: &mut usize) -> u32 {
    let mut current_value: u32 = 0;
    let mut shift_amount: u8 = 0;

    while *pos < slice.len() {
        let current_byte = slice[*pos];
        let mask_result = VALUE_MASK & current_byte;
        current_value |= (mask_result as u32) << shift_amount;

        *pos += 1;
        if (CONTINUATION_MASK & current_byte) != 0 {
            break;
        }
        shift_amount += 7;
    }

    current_value
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::{decode_var_int, CONTINUATION_MASK};

    #[test]
    fn test_decode() {
        let mut pos = 0;

        assert_eq!(
            decode_var_int(&[], &mut pos),
            0
        );
        assert_eq!(0, pos);

        pos = 0;
        assert_eq!(
            decode_var_int(&[CONTINUATION_MASK | 0], &mut pos),
            0
        );
        assert_eq!(1, pos);

        pos = 0;
        assert_eq!(
            decode_var_int(&[CONTINUATION_MASK | 64], &mut pos),
            64
        );
        assert_eq!(1, pos);

        pos = 0;
        assert_eq!(
            decode_var_int(&[CONTINUATION_MASK | 127], &mut pos),
            127
        );
        assert_eq!(1, pos);

        pos = 0;
        assert_eq!(
            decode_var_int(&[0, CONTINUATION_MASK | 127], &mut pos),
            16256
        );
        assert_eq!(2, pos);

        pos = 0;
        assert_eq!(
            decode_var_int(&[10, CONTINUATION_MASK | 127], &mut pos),
            16266
        );
        assert_eq!(2, pos);

        pos = 0;
        assert_eq!(
            decode_var_int(&[127, CONTINUATION_MASK | 127], &mut pos),
            16383
        );
        assert_eq!(2, pos);

        pos = 0;
        assert_eq!(
            decode_var_int(&[127, 127, 127, 127, CONTINUATION_MASK | 15], &mut pos),
            u32::MAX
        );
        assert_eq!(5, pos);
    }
}
