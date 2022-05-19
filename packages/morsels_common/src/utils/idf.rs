#[inline(always)]
pub fn get_idf(num_docs: f32, doc_freq: f32) -> f32 {
    (1.0 + (num_docs - doc_freq + 0.5) / (doc_freq + 0.5)).ln()
}
