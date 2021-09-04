#[inline(always)]
pub fn check(vec: &[u8], at: usize) -> bool {
    let byte_number = at / 8;
    let bit_number = 1_u8 << (at % 8) as u8;
    (vec[byte_number] & bit_number) != 0
}

#[cfg(test)]
mod test {
    use super::check;

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
}
