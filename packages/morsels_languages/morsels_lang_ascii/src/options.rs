use miniserde::json::{self, Object};

pub fn get_stop_words(options: &Object) -> Option<Vec<String>> {
    options.get("stop_words")
        .map_or(
            None,
            |v| if let json::Value::Array(arr) = v {
                Some(
                    arr.iter()
                        .filter_map(|s| if let json::Value::String(s) = s {
                            Some(s.to_owned())
                        } else {
                            None
                        })
                        .collect()
                )
            } else {
                None
            }
        )
}

pub fn get_ignore_stop_words(options: &Object) -> bool {
    options.get("ignore_stop_words")
        .map_or(
            false,
            |v| if let json::Value::Bool(ignore_stop_words) = v {
                *ignore_stop_words
            } else {
                false
            }
        )
}

pub fn get_max_term_len(options: &Object) -> usize {
    options.get("max_term_len")
        .map_or(80, |v| if let json::Value::Number(json::Number::U64(n)) = v {
            *n as usize
        } else {
            80
        })
}
