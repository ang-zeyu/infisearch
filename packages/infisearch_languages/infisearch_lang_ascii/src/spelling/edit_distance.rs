/*
The MIT License (MIT)

Copyright (c) 2015 Danny Guo
Copyright (c) 2016 Titus Wormer <tituswormer@gmail.com>
Copyright (c) 2018 Akash Kurdekar

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

/*
 Modified and optimized from strsim-rs for Morsels' use case:
 Avoid unnecessary Vec allocation,
 reduces data sizes (Vec -> [; 255]) for the term length hard limit (guarantees unsafe too),
 and remove an unnecessary empty term check.
 */

#[inline]
pub fn levenshtein(
    a: &str,
    b: &str,
    b_len: usize,
    cache: &mut [usize],
) -> usize {
    for i in 0..b_len {
        unsafe { *cache.get_unchecked_mut(i) = i + 1; }
    }

    let mut result = 0_usize;

    for (i, a_elem) in a.chars().enumerate() {
        result = i + 1;
        let mut distance_b = i;

        for (j, b_elem) in b.chars().enumerate() {
            let cost = if a_elem == b_elem { 0usize } else { 1usize };
            let distance_a = distance_b + cost;
            distance_b = unsafe { *cache.get_unchecked(j) };
            result = std::cmp::min(result + 1, std::cmp::min(distance_a, distance_b + 1));
            unsafe {
                *cache.get_unchecked_mut(j) = result;
            }
        }
    }

    result
}
