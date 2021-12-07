#[inline(always)]
pub fn get_idf(num_docs: f64, doc_freq: f64) -> f64 {
    (1.0 + (num_docs - doc_freq + 0.5) / (doc_freq + 0.5)).ln()
}
