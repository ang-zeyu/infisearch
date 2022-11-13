// https://doc.rust-lang.org/src/alloc/vec/mod.rs.html#1764
// Without the growing, for minimizing code size.

use std::ptr;

#[inline(always)]
pub fn push_wo_grow<T>(vec: &mut Vec<T>, val: T) {
    unsafe {
        let len = vec.len();
        let end = vec.as_mut_ptr().add(len);
        ptr::write(end, val);
        vec.set_len(len + 1);
    }
}
