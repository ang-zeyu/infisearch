# Language Configuration

There are 3 language modules available. To configure these, you will need to serve the appropriate [language bundle](./getting_started.md#hosting-the-files) in your HTML (or edit the CDN link accordingly), and edit the indexer configuration file.

```json
{
  "lang_config": {
    // ... options go here ...
  }
}
```

## Ascii Tokenizer

The default tokenizer should work for any language that relies on ASCII characters, or their inflections (e.g. "á").

The text is first split into on sentences, then whitespaces to obtain tokens. An [asciiFoldingFilter](https://github.com/tantivy-search/tantivy/blob/main/src/tokenizer/ascii_folding_filter.rs) is then applied to normalize diacritics, followed by punctuation and non-word-character boundary removal.

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
<script src="https://cdn.jsdelivr.net/gh/ang-zeyu/infisearch@v0.9.1/packages/search-ui/dist/search-ui.ascii.bundle.js"></script>
```

## Ascii Tokenizer with Stemmer

This is essentially the same as the ascii tokenizer, but adds a `stemmer` option.

```json
{
  "lang": "ascii_stemmer",
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
<script src="https://cdn.jsdelivr.net/gh/ang-zeyu/infisearch@v0.9.1/packages/search-ui/dist/search-ui.ascii-stemmer.bundle.js"></script>
```

## Chinese Tokenizer

This is a lightweight character-wise tokenizer, **not** based on word-based tokenizers like Jieba.

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
<script src="https://cdn.jsdelivr.net/gh/ang-zeyu/infisearch@v0.9.1/packages/search-ui/dist/search-ui.chinese.bundle.js"></script>
```

## Stop Words

All tokenizers support keeping (default) or removing stop words using the `ignore_stop_words` option.

Keeping them enables the following:
- Processing phrase queries such as `"for tomorrow"` accurately; Stop words would be removed automatically from such queries.
- Boolean queries of stop words (e.g. `if AND forecast AND sunny`)
- More accurate ranking for free text queries, which uses stop words in term proximity ranking

## UI Translations

The UI's text can also be overwritten.
Refer to this [link](https://github.com/ang-zeyu/infisearch/tree/main/packages/search-ui/src/translations/en.ts) for the default set of texts.

```ts
infisearch.init({
  uiOptions: {
    translations: { ... }
  }
})
```

| Option                | Default                 | Description |
| -----------           | -----------             | ----------- |
| `resultsLabel`        | `'Site results'`        | Accessibility label for the [`listbox`](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/listbox_role) containing result previews. This is announced to screenreaders.
| `fsButtonLabel`        | `'Search'` | Accessibility label for the original input element that functions as a button when the fullscreen UI is in use.
| `fsButtonPlaceholder`        | `undefined`| Placeholder override for the provided `input` that functions as a button when the fullscreen UI is in use.
| `fsPlaceholder` | `'Search this site'` | Placeholder of the input element in the fullscreen UI.
| `fsCloseText` | `'Close'` | Text for the <kbd>Close</kbd> button.
| `filtersButton` | `'Filters'` | Text for the <kbd>Filters</kbd> button if any enum or numeric filters are configured.
| `numResultsFound`        | `' results found'` | The text following the number of results found.
| `startSearching`        | `'Start Searching Above!'`| Text shown when the input is empty.
| `startingUp`       | `'... Starting Up ...'` | Text shown when InfiSearch is still not ready to perform any queries. The setup occurs extremely quickly, you will hopefully not be able to see this text most of the time.
| `navigation`       | `'Navigation'` | Navigation controls text.
| `sortBy`       | `'Sort by'` | Header text for custom [sort orders](./search_configuration.md#setting-up-numeric-filters-and-sort-orders).
| `tipHeader`       | `'🔎 Advanced search tips'` | Header of the tip popup.
| `tip`       | `'Tip'` | First column header of the tip popup.
| `example`       | `'Example'` | Second column header of the tip popup.
| `tipRows.xx` (refer [here](https://github.com/ang-zeyu/infisearch/tree/main/packages/search-ui/src/translations/en.ts)) |  | Examples for usage of InfiSearch's advanced search syntax.
| `error`         | `'Oops! Something went wrong... 🙁'` | Generic error text when something goes wrong
