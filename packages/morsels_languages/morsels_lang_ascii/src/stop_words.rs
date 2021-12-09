use std::collections::HashSet;

fn get_default_stop_words() -> Vec<String> {
    vec![
        // Same list from tantivy
        "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is", "it", "no",
        "not", "of", "on", "or", "such", "that", "the", "their", "then", "there", "these", "they", "this",
        "to", "was", "will", "with",
    ]
    .into_iter()
    .map(|s| s.to_owned())
    .collect()
}

pub fn get_stop_words_set(stop_words_vec: Vec<String>) -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::default();

    for word in stop_words_vec {
        set.insert(word.to_owned());
    }

    set
}

pub fn get_default_stop_words_set() -> HashSet<String> {
    get_stop_words_set(get_default_stop_words())
}
