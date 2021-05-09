use std::convert::TryInto;

static VALUE_MASK: u32 = 0x7f; // 0111 1111
static CONTINUATION_MASK: u8 = 0x80; // 1000 0000

pub fn get_var_int(value: u32) -> Vec<u8> {
    let mut buffer = Vec::new();

    let mut new_value = value;
    loop {
        let last_seven_bits: u8 = (new_value & VALUE_MASK).try_into().unwrap();
        new_value >>= 7;

        if new_value > 0 {
            buffer.push(last_seven_bits);
        } else {
            buffer.push(last_seven_bits & CONTINUATION_MASK);
            break;
        }
    }

    buffer
}
