# Language Configuration

The snippet below shows the default values for language configuration. The main tokenizer module is specified by `lang`, while the `options` key supplies tokenization options unique to each language module.

> These options are also applied at search time, which is retrieved from a metadata file in the index output directory.

```json
{
  "lang_config": {
    "lang": "ascii",
    "options": {
      // Language dependent
    }
  }
}
```

Only the following 3 tokenizers are supported for now:

## Ascii Tokenizer

The default tokenizer splits on sentences, then whitespaces to obtain tokens.

An [asciiFoldingFilter](https://github.com/tantivy-search/tantivy/blob/main/src/tokenizer/ascii_folding_filter.rs) is then applied to these tokens, followed by punctuation and non-word boundary removal.

```json
{
  "lang": "ascii",
  "options": {
    "stop_words": [
      "a", "an", "and", "are", "as", "at", "be", "but", "by", "for",
      "if", "in", "into", "is", "it", "no", "not", "of", "on", "or",
      "such", "that", "the", "their", "then", "there", "these",
      "they", "this", "to", "was", "will", "with"
    ],
    "ignore_stop_words": true,

    // Hard limit = 250
    "max_term_len": 80
  }
}
```

## Latin Tokenizer

This is essentially the same as the ascii tokenizer, but adds a `stemmer` option.

```json
{
  "lang": "latin",
  "options": {
    // ----------------------------------
    // Ascii Tokenizer options also apply
    // ...
    // ----------------------------------

    // Any of the languages here
    // https://docs.rs/rust-stemmers/1.2.0/rust_stemmers/enum.Algorithm.html
    // Languages other than "english" have not been extensively tested. Use with caution!
    "stemmer": "english"
  }
}
```

If you do not need stemming, use the `ascii` tokenizer, which has a smaller wasm binary.

## Chinese Tokenizer

A basic `chinese` tokenizer based on [jieba-rs](https://github.com/messense/jieba-rs) is also available, although, it hasn't been extensively tested. Use with caution!

This tokenizer applies jieba's `cut` method to obtain various tokens, then applies a punctuation filter to these tokens. Thereafter, tokens are grouped into sentences.

```json
{
  "lang": "chinese",
  "options": {
    "stop_words": [],
    "ignore_stop_words": true
  }
}
```

## Stop Words

All tokenizers support keeping or removing (default) stop words using the `ignore_stop_words` option.

Keeping them enables the following:
- Processing phrase queries such as `"for tomorrow"`
- Boolean queries of stop words (e.g. `if AND forecast AND sunny`)
- More accurate ranking for free text queries, which employ an inverse document frequency heuristic to prune stop words only when their impact is small (far from always the case!). 
