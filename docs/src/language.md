# Language Configuration

There are 3 language modules available. To configure these, you will need to serve the appropriate [language bundle](./getting_started.md#hosting-the-files) in your HTML (or edit the CDN link accordingly), and edit the indexer configuration file.

## Ascii Tokenizer

#### CDN link

The default tokenizer splits on sentences, then whitespaces to obtain tokens.

An [asciiFoldingFilter](https://github.com/tantivy-search/tantivy/blob/main/src/tokenizer/ascii_folding_filter.rs) is then applied to these tokens, followed by punctuation and non-word-character boundary removal.

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
    "ignore_stop_words": false,

    // Hard limit = 250
    "max_term_len": 80
  }
}
```

**CDN Link**

```html
<script src="https://cdn.jsdelivr.net/gh/ang-zeyu/morsels@v0.4.1/packages/search-ui/dist/search-ui.ascii.bundle.js"></script>
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

**CDN Link**

```html
<script src="https://cdn.jsdelivr.net/gh/ang-zeyu/morsels@v0.4.1/packages/search-ui/dist/search-ui.latin.bundle.js"></script>
```

## Chinese Tokenizer

This is a lightweight and experimental character-wise tokenizer, **not** based on word-based tokenizers like jieba.

You are highly recommended to keep positions indexed and query term proximity ranking turned on when using this tokenizer, in order to boost the relevance of documents with multi-character queries.

```json
{
  "lang": "chinese",
  "options": {
    "stop_words": [],
    "ignore_stop_words": false,
    "max_term_len": 80
  }
}
```

**CDN Link**

```html
<script src="https://cdn.jsdelivr.net/gh/ang-zeyu/morsels@v0.4.1/packages/search-ui/dist/search-ui.chinese.bundle.js"></script>
```

## Stop Words

All tokenizers support keeping (default) or removing stop words using the `ignore_stop_words` option.

Keeping them enables the following:
- Processing phrase queries such as `"for tomorrow"` accurately; Stop words would be removed automatically from such queries.
- Boolean queries of stop words (e.g. `if AND forecast AND sunny`)
- More accurate ranking for free text queries, which uses stop words in term proximity ranking, and removes them only when their impact is small.
