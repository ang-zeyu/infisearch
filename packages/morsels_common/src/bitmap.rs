#[inline(always)]
pub fn check(vec: &[u8], at: usize) -> bool {
    let byte_number = at / 8;
    let bit_number = 1_u8 << (at % 8) as u8;
    (vec[byte_number] & bit_number) != 0
}
