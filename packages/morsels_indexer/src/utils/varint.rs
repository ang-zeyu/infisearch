static VALUE_MASK: u32 = 127; // 0111 1111
static CONTINUATION_MASK: u8 = 128; // 1000 0000

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
