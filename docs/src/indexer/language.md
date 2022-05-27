# `lang_config`

The snippet below shows the default values for language configuration. The key controlling the main tokenizer module to use is `lang`, while the `options` key supplies tokenization options unique to each module.

> These options are also applied to the search user interface and library where appropriate, and is stored in a metadata file in the index output directory.

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
    // For example, "english"
    "stemmer": "english"
  }
}
```

It is separated from the ascii tokenizer to reduce binary size (about ~`220KB` savings before gzip).

## Chinese Tokenizer

A basic `chinese` tokenizer based on [jieba-rs](https://github.com/messense/jieba-rs) is also available, although, it hasn't been as extensively tested. Use at your own discretion!

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

All tokenizers support forcibly removing stop words using the `ignore_stop_words` option, should you wish to keep the index size down.

Keeping stop words enables the following:
- Processing phrase queries such as `"for tomorrow"`
- Boolean queries of stop words (e.g. `if AND forecast AND sunny`)
- More accurate ranking for free text queries, which employ an inverse document frequency heuristic to prune stop words only when their impact is small (far from always the case!). 

> If you are using any of the 2 `large` presets covered in [section 5.4](./presets.md), which generates a sharded index, stop words are not removed by default. This is because these options split up the index, which means that such commonly occuring words are likely to be separately placed into one file. (and never requested until necessary)


## Note on Language Modules' Flexibility

While using the same tokenizer for both indexing / search unifies the codebase, one downside is that code size has to be taken into account.

The chinese tokenizer for example, which uses *jieba-rs*, accounts for half of the wasm binary size alone.

Therefore, the tokenizers will aim to be reasonably powerful and configurable enough, such that the wasm bundle size dosen't blow up.

Nonetheless, if you feel that a certain configuration option should be supported for a given tokenizer but isn't, feel free to open up a feature request!
