#[inline(always)]
pub fn check(vec: &[u8], at: usize) -> bool {
    let byte_number = at / 8;
    let bit_number = 1_u8 << (at % 8) as u8;
    (vec[byte_number] & bit_number) != 0
}

#[inline(always)]
pub fn set(vec: &mut [u8], at: usize) {
    let byte_num = at / 8_usize;
    vec[byte_num] |= 1_u8 << (at % 8) as u8;
}

#[inline(always)]
pub fn read_bits_from(bit_pos: &mut usize, mut bit_len: usize, buf: &[u8]) -> u32 {
    let mut v: u32 = 0;

    loop {
        let byte_number = *bit_pos / 8;
        let bit_offset_from_end = 8 - (*bit_pos % 8);

        let bits_this_byte = bit_offset_from_end.min(bit_len);

        debug_assert!(bits_this_byte <= bit_offset_from_end);

        let shift = (bit_len - bits_this_byte) as u32;

        if bits_this_byte == 8 {
            v += (buf[byte_number] as u32) << shift;
        } else {
            let mask = (1_u8 << bits_this_byte) - 1;
            v += (
                ((buf[byte_number] >> (bit_offset_from_end - bits_this_byte)) & mask) as u32
            ) << shift;
        }

        *bit_pos += bits_this_byte;
        if bit_len <= bits_this_byte {
            break;
        }

        bit_len -= bits_this_byte;
    }

    v
}

#[cfg(test)]
mod test {
    use super::{check, set, read_bits_from};

    #[test]
    fn test_bitmap() {
        assert!(!check(&[0], 0));
        assert!(check(&[1], 0));
        assert!(check(&[129], 0));
        assert!(check(&[129], 7));
        assert!(check(&[0, 129], 8));
        assert!(!check(&[0, 129], 9));
        assert!(check(&[0, 129], 15));
    }

    #[test]
    fn test_bitmap_set() {
        let mut vec = vec![0; 10];
        set(&mut vec, 0);
        assert!(check(&vec, 0));

        vec = vec![0; 10];
        set(&mut vec, 7);
        assert!(check(&vec, 7));

        vec = vec![0; 10];
        set(&mut vec, 8);
        assert!(check(&vec, 8));

        vec = vec![0; 10];
        set(&mut vec, 9);
        assert!(check(&vec, 9));

        vec = vec![0; 10];
        set(&mut vec, 15);
        assert!(check(&vec, 15));
    }

    #[test]
    fn test_bitread() {
        let mut bit_pos = 0;
        let result = read_bits_from(&mut bit_pos, 5, &[0, 0, 0]);
        assert!(bit_pos == 5);
        assert!(result == 0);

        let mut bit_pos = 0;
        let result = read_bits_from(&mut bit_pos, 15, &[0, 0, 0]);
        assert!(bit_pos == 15);
        assert!(result == 0);

        let mut bit_pos = 0;
        let result = read_bits_from(&mut bit_pos, 8, &[255, 0, 0]);
        assert!(bit_pos == 8);
        assert!(result == 255);
        
        let mut bit_pos = 0;
        let result = read_bits_from(&mut bit_pos, 4, &[255, 0, 0]);
        assert!(bit_pos == 4);
        assert!(result == 15);
        
        let mut bit_pos = 11;
        let result = read_bits_from(&mut bit_pos, 4, &[0, 255, 0]);
        assert!(bit_pos == 15);
        assert!(result == 15);

        let mut bit_pos = 0;
        let result = read_bits_from(&mut bit_pos, 11, &[255, 0b0010_0000, 0]);
        assert!(bit_pos == 11);
        assert!(result == 0b1111_1111_001);

        let mut bit_pos = 0;
        let result = read_bits_from(&mut bit_pos, 24, &[255, 0b0010_0000, 0]);
        assert!(bit_pos == 24);
        assert!(result == 0b1111_1111_0010_0000_0000_0000);

        let mut bit_pos = 4;
        let result = read_bits_from(&mut bit_pos, 11, &[255, 0b0010_0000, 0]);
        assert!(bit_pos == 15);
        assert!(result == 0b1111_0010_000);

        let mut bit_pos = 4;
        let result = read_bits_from(&mut bit_pos, 20, &[255, 0b0010_0000, 0]);
        assert!(bit_pos == 24);
        assert!(result == 0b1111_0010_0000_0000_0000);
    }
}
