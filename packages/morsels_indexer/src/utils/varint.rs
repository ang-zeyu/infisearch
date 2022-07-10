const VALUE_MASK: u32 = 127; // 0111 1111
const CONTINUATION_MASK: u8 = 128; // 1000 0000

pub fn get_var_int(mut value: u32, output_buf: &mut [u8]) -> &[u8] {
    for buf_idx in 0..16 {
        let last_seven_bits: u8 = (value & VALUE_MASK) as u8;
        value >>= 7;

        if value != 0 {
            output_buf[buf_idx] = last_seven_bits;
        } else {
            output_buf[buf_idx] = last_seven_bits | CONTINUATION_MASK;
            return &output_buf[..buf_idx + 1];
        }
    }

    panic!("Attempted to encode variable integer over 16 bytes in length!");
}

pub fn get_var_int_vec(mut value: u32, output_buf: &mut Vec<u8>) {
    for _buf_idx in 0..16 {
        let last_seven_bits: u8 = (value & VALUE_MASK) as u8;
        value >>= 7;

        if value != 0 {
            output_buf.push(last_seven_bits);
        } else {
            output_buf.push(last_seven_bits | CONTINUATION_MASK);
            return;
        }
    }

    panic!("Attempted to encode variable integer over 16 bytes in length!");
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::{get_var_int, CONTINUATION_MASK};

    #[test]
    fn test_encode() {
        let mut output_buf: [u8; 16] = [0; 16];

        assert_eq!(get_var_int(0, &mut output_buf), &[CONTINUATION_MASK | 0]);

        assert_eq!(get_var_int(64, &mut output_buf), &[CONTINUATION_MASK | 64]);

        assert_eq!(get_var_int(127, &mut output_buf), &[CONTINUATION_MASK | 127]);

        assert_eq!(get_var_int(16256, &mut output_buf), &[0, CONTINUATION_MASK | 127]);

        assert_eq!(get_var_int(16266, &mut output_buf), &[10, CONTINUATION_MASK | 127]);

        assert_eq!(get_var_int(16383, &mut output_buf), &[127, CONTINUATION_MASK | 127]);

        assert_eq!(get_var_int(u32::MAX, &mut output_buf), &[127, 127, 127, 127, CONTINUATION_MASK | 15]);
    }
}
