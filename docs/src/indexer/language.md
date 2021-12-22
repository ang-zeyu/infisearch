# `lang_config`

The snippet below shows the default values for language configuration. The key controlling the main tokenizer module to use is the `lang` key, while the `options` key supplies tokenization options unique to each module.

> These options are also applied to `@morsels/search-ui`, which sources this information from some metadata available in the index output directory.

```json
{
  "lang_config": {
    "lang": "ascii",
    "options": null
  }
}
```

## Ascii Tokenizer

The default tokenizer splits on sentences, then whitespaces to obtain tokens.

An [asciiFoldingFilter](https://github.com/tantivy-search/tantivy/blob/main/src/tokenizer/ascii_folding_filter.rs) is then applied to these tokens, followed by punctuation and non-word boundary removal.

```json
"lang_config": {
  "lang": "latin",
  "options": {
    "stop_words": [
      "a", "an", "and", "are", "as", "at", "be", "but", "by", "for",
      "if", "in", "into", "is", "it", "no", "not", "of", "on", "or",
      "such", "that", "the", "their", "then", "there", "these",
      "they", "this", "to", "was", "will", "with"
    ],
    "ignore_stop_words": false,

    "max_term_len": 80
  }
}
```

## Latin Tokenizer

This is essentially the same as the ascii tokenizer, but adds a `stemmer` option.

```
"lang_config": {
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

A basic `chinese` tokenizer based on [jieba-rs](https://github.com/messense/jieba-rs) is also available, although, it is still a heavy WIP at the moment. Use at your own discretion.

This tokenizer applies jieba's `cut` method to obtain various tokens, then applies a punctuation filter to these tokens. Thereafter, tokens are grouped into sentences.

```json
"lang_config": {
  "lang": "chinese",
  "options": {
    "stop_words": [],
    "ignore_stop_words": false
  }
}
```

## Note on Stop Words

A slightly different approach with stop words is taken **by default** in that stop words are only filtered at **query time** for certain types of queries. Currently, this is for free-text queries with more than two terms, since the inverse document frequency of such terms are likely to have become negligible compared to other terms in the query at this point.

Moreover, splitting up the index means that such commonly occuring words are likely to be completely and separately placed into one file. This means that information for stop words is never requested unless necessary:
- For processing phrase queries (eg. `"for tomorrow"`)
- Boolean queries (eg. `if AND forecast AND sunny`)
- One or two term free text queries containing stop words only. This is an unlikely use case, but it is nice having some results show up than none.

Nevertheless, all tokenizers also support forcibly removing stop words using the `ignore_stop_words` option, should you wish to keep the index size down (discussed again under chapter on ["Tradeoffs"](../tradeoffs.md)).


## Note on Language Modules' Flexibility

While using the same tokenizer for both indexing / search unifies the codebase, one downside is that code size has to be taken into account.

The chinese tokenizer for example, which uses *jieba-rs*, accounts for half of the wasm binary size alone.

Therefore, the tokenizers will aim to be reasonably powerful and configurable enough, such that the wasm bundle size dosen't blow up.

Nonetheless, if you feel that a certain configuration option should be supported for a given tokenizer but isn't, feel free to open up a feature request!
