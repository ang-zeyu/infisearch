static VALUE_MASK: u8 = 0x7f; // 0111 1111
static CONTINUATION_MASK: u8 = 0x80; // 1000 0000

pub fn decode_var_int(slice: &[u8]) -> (u32, usize) {
  let mut current_value: u32 = 0;
  let mut shift_amount: u8 = 0;
  let mut pos = 0;

  while pos < slice.len() {
    let current_byte = slice[pos];
    let mask_result = VALUE_MASK & current_byte;
    current_value += (mask_result as u32) << shift_amount;

    if (CONTINUATION_MASK & current_byte) != 0 {
      pos += 1;
      break;
    } else {
      shift_amount += 7;
    }

    pos += 1
  }

  (current_value, pos)
}
