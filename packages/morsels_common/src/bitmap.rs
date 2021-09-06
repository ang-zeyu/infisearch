#[inline(always)]
pub fn check(vec: &[u8], at: usize) -> bool {
    let byte_number = at / 8;
    let bit_number = 1_u8 << (at % 8) as u8;
    (vec[byte_number] & bit_number) != 0
}

#[inline(always)]
pub fn set(vec: &mut[u8], at: usize) {
    let byte_num = at / 8 as usize;
    vec[byte_num] |= 1_u8 << (at % 8) as u8;
}

#[cfg(test)]
mod test {
    use super::{check, set};

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
}
