use morsels_common::MorselsLanguageConfig;

#[cfg(feature = "indexer")]
use std::collections::HashSet;

#[cfg(feature = "indexer")]
fn get_stop_words_set<'a, T: IntoIterator<Item = &'a str>>(stop_words: T) -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::default();

    for word in stop_words {
        set.insert(word.to_owned());
    }

    set
}

#[cfg(feature = "indexer")]
pub fn get_stop_words(lang_config: &MorselsLanguageConfig, defaults: &[&'static str]) -> HashSet<String> {
    if let Some(stop_words) = &lang_config.options.stop_words {
        get_stop_words_set(stop_words.into_iter().map(|s| s.as_str()))
    } else {
        get_stop_words_set(defaults.into_iter().map(|s| *s))
    }
}

#[cfg(not(feature = "indexer"))]
pub fn get_stop_words(lang_config: &MorselsLanguageConfig, defaults: &[&'static str]) -> Vec<String> {
    if let Some(stop_words) = &lang_config.options.stop_words {
        stop_words.clone()
    } else {
        defaults.into_iter().map(|s| (*s).to_owned()).collect()
    }
}
