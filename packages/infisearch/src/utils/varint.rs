use bitvec::{prelude::BitVec, order::Msb0, view::BitView};

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

pub fn get_var_int_vec_u64(mut value: u64, output_buf: &mut Vec<u8>) {
    for _buf_idx in 0..16 {
        let last_seven_bits: u8 = (value & 127) as u8;
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


struct PackedVarIntUnit {
    t: usize,
    v: u32,
    do_write_len: bool,
    len: u8,
    from: usize,
}

/// Bitwise compressor for a set of uniformly,
/// repeatedly occuring (1,2,3,4,1,2,3,4,...) integers.
/// 
/// Writes are all lazy until flush() is called,
/// as we need to know the patterns of the values.
pub struct PackedVarIntWriter<const NUM_TYPES: usize>
{
    // Parameter: Maximum bits the current chunk can use to encode an integer
    max_bit_lens: [usize; NUM_TYPES],
    // Parameter: Maximum number of integers one of each type's chunk can hold
    max_values: [usize; NUM_TYPES],

    // Temporary storage of (type, value) tuples in write-order
    type_chunk_buffers: Vec<PackedVarIntUnit>,

    bit_vec: BitVec<u8, Msb0>,
}

impl<const NUM_TYPES: usize> PackedVarIntWriter<NUM_TYPES> {
    pub fn new(
        mut max_bit_lens: [usize; NUM_TYPES],
        max_values: [usize; NUM_TYPES],
    ) -> PackedVarIntWriter<NUM_TYPES> {
        for max_bit_len in max_bit_lens.iter_mut() {
            *max_bit_len = 8 - *max_bit_len;
        }

        PackedVarIntWriter {
            max_bit_lens,
            max_values,
            type_chunk_buffers: Vec::new(),
            bit_vec: BitVec::new(),
        }
    }

    pub fn write_type(&mut self, t: usize, v: u32) {
        #[cfg(debug_assertions)]
        {
            // How many bits to encode the value?
            let min_bits = (v as f64).log2() as u8 + 1;

            // How many bits to encode the above?
            let min_min_bits = (min_bits as f64).log2() as u8 + 1;

            debug_assert!(min_min_bits as usize <= (8 - self.max_bit_lens[t]));
        }

        self.type_chunk_buffers.push(PackedVarIntUnit {
            t,
            v,

            // ---------------------------
            // Populated during flush()
            do_write_len: false,
            len: 0,
            from: 0,
        });
    }

    pub fn flush(mut self) -> BitVec<u8, Msb0> {

        for t in 0..NUM_TYPES {
            let mut for_chunking = Vec::new();
            for_chunking.extend(self.type_chunk_buffers.iter_mut().filter(|unit| unit.t == t));

            for units in for_chunking.chunks_mut(self.max_values[t]) {
                let chunk_max = units.iter().map(|unit| unit.v).max().unwrap();

                let min_bits = if chunk_max == 0 {
                    // possible due to doc_freq == 0 acting as a new postings list delimeter
                    // or 0 prefix length
                    // etc.
                    1
                } else {
                    (chunk_max as f64).log2() as u8 + 1
                };
    
                debug_assert!(min_bits > 0);
                debug_assert!(min_bits <= 32);

                units.first_mut().unwrap().do_write_len = true;

                let from = 32 - min_bits as usize;
                for unit in units.iter_mut() {
                    unit.len = min_bits - 1;  // + 1-ed when decoding again. To allow encoding 1 more power.
                    unit.from = from;
                }
            }
        }

        for unit in self.type_chunk_buffers {
            if unit.do_write_len {
                // Also write the chunk's bit length for the first element of each chunk
                self.bit_vec.extend(
                    &unit.len.view_bits::<Msb0>()[self.max_bit_lens[unit.t]..]
                );
            }

            self.bit_vec.extend(&unit.v.view_bits::<Msb0>()[unit.from..]);
        }

        self.bit_vec
    }
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
