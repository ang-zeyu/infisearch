# Indexer Misc Configuration

## Indexing Positions

```json
{
  "indexing_config": {
    "with_positions": true
  }
}
```

This option controls if positions are stored.
Features such as phrase queries that require positional information will not work if this is disabled.
Turning this off for very large collections (~> 1GB) can increase the tool's scalability, at the cost of such features.

## Indexer Thread Count

```json
{
  "indexing_config": {
    "num_threads": max(min(physical cores, logical cores) - 1, 1)
  }
}
```

## Indexing Multiple Files Under One Document

InfiSearch regards each file as a single document by default. You can index **multiple files** into **one document** using the reserved field [`_add_files`](./fields.md#reserved-fields). This is useful if you need to override or add data but can't modify the source document easily.

Overrides should be provided with JSON, CSV, or HTML files, as TXT and PDF files have no reliable way of supplying the `_add_files` field. In addition, you will need to manually map the CSV data to the `_add_files` field. This is automatically done for JSON and [HTML](../linking_to_others.md) files.

#### Example: Overriding a Document's Link With Another File

Suppose you have the following files:

```
folder
|-- main.html
|-- overrides.json
```

To index `main.html` and override its link, you would have:

`overrides.json`

```json
{
  "link": "https://infi-search.com",
  "_add_files": "./main.html"
}
```

Indexer Configuration

```json
{
  "indexing_config": {
    "exclude": ["main.html"]
  }
}
```

This excludes indexing `main.html` directly, but does so through `overrides.json`.

## Larger Collections

> ⚠️ This section serves as a reference, prefer the preconfigured [scaling presets](../larger_collections.md) if possible.

**Field Configuration**

```json
{
  "fields_config": {
    "cache_all_field_stores": true,
    "num_docs_per_store": 100000000
  },
  "indexing_config": {
    "pl_limit": 4294967295,
    "pl_cache_threshold": 0,
    "num_pls_per_dir": 1000
  }
}
```

#### Field Store Caching: **`cache_all_field_stores`**

All fields specified with `storage=[{ "type": "text" }]` are cached up front when this is enabled.
This is the same option as the one under [search functionality options](../search_configuration.md#search-functionality-options), and has lower priority.

#### Field Store Granularity: `num_docs_per_store`

The `num_docs_per_store` parameter controls how many documents' texts to store in one JSON file. Batching multiple files together increases file size but can lead to less files and better browser caching.

#### Index Shard Size: **`pl_limit`**

This is a threshold (in bytes) at which to "cut" index (**pl** meaning [postings list](https://en.wikipedia.org/wiki/Inverted_index)) chunks.
Increasing this produces less but bigger chunks (which take longer to retrieve).

#### Index Caching: **`pl_cache_threshold`**

Index chunks that exceed this size (in bytes) are cached by the search library on initilisation.
It is used to configure InfiSearch for response time (over scalability) for typical use cases.
