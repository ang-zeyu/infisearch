pub struct PackedVarIntReader<'a, const NUM_TYPES: usize>
{
    // Parameter: How many integers does the chunk encode, maximum 255
    type_chunk_max_idxes: [u8; NUM_TYPES],
    // Parameter: Maximum bits the current chunk can use to encode an integer
    max_bit_lens: [usize; NUM_TYPES],

    // How many bits does the current chunk use to encode an integer
    type_chunk_lens: [usize; NUM_TYPES],
    // Index of the integer in the current chunk, maximum 254
    type_chunk_idxes: [u8; NUM_TYPES],

    bit_pos: usize,
    bits_as_bytes: &'a [u8],
}

impl<'s, const NUM_TYPES: usize> PackedVarIntReader<'s, NUM_TYPES> {
    pub fn new(
        slice: &[u8],
        max_bit_lens: [usize; NUM_TYPES],
        type_chunk_max_idxes: [u8; NUM_TYPES],
    ) -> PackedVarIntReader<NUM_TYPES> {
        PackedVarIntReader {
            max_bit_lens,
            type_chunk_max_idxes,
            type_chunk_lens: [0; NUM_TYPES],
            type_chunk_idxes: [0; NUM_TYPES],
            bit_pos: 0,
            bits_as_bytes: slice,
        }
    }

    pub fn read_type(&mut self, t: usize) -> u32 {
        // Guarantees: t < NUM_TYPES at compile time

        if unsafe { *self.type_chunk_idxes.get_unchecked(t) } == 0 {
            // Read current chunk's bit length
            *unsafe { self.type_chunk_lens.get_unchecked_mut(t) } = read_bits_from(
                &mut self.bit_pos,
                unsafe { *self.max_bit_lens.get_unchecked(t) },
                &self.bits_as_bytes,
            ) as usize + 1; // was -1 ed when encoding to allow encoding 1 more power, reverse it
        }

        unsafe {
            *self.type_chunk_idxes.get_unchecked_mut(t) =
                ((*self.type_chunk_idxes.get_unchecked(t)) + 1)
                % (*self.type_chunk_max_idxes.get_unchecked(t));
        }

        read_bits_from(
            &mut self.bit_pos,
            unsafe { *self.type_chunk_lens.get_unchecked(t) },
            &self.bits_as_bytes,
        )
    }
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

        debug_assert!(byte_number < buf.len());

        if bits_this_byte == 8 {
            v += (unsafe { *buf.get_unchecked(byte_number) } as u32) << shift;
        } else {
            let mask = (1_u8 << bits_this_byte) - 1;
            v += (
                ((unsafe { *buf.get_unchecked(byte_number) } >> (bit_offset_from_end - bits_this_byte)) & mask) as u32
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
    use super::read_bits_from;

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
