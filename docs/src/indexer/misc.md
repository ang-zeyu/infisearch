# Indexer Misc Configuration

## Indexing Positions

**`with_positions = true`**

This option controls whether positions will be stored.

Features such as phrase queries that require positional information will not work if this is disabled.

Turning this off for very large collections (~> 1GB) can increase the tool's scalability, at the cost of such features.

## Indexer Performance

```json
{
  "indexing_config": {
    "num_threads": <number of physical cpus> - 1
  }
}
```

This is the number of threads to use, excluding the main thread. When unspecified, this is `max(min(num physical cores, num logical cores) - 1, 1)`.


## Indexing Multiple Files Under One Document

InfiSearch regards each file as a single document by default. You can index **multiple files** into **one document** using the reserved field [`_add_files`](./fields.md#reserved-fields). This is useful if you need to **override or add data** but can't modify the source document easily.

#### Example: Overriding a Document's Title

Suppose you have the following files:

```
folder
|-- main.html
|-- overrides.json
```

To index `main.html` and override its title, you would have:

Inside `overrides.json`,

```json
{
  "title": "Title Override",
  "_add_files": "./main.html"
}
```

And inside your configuration file:

```json
{
  "indexing_config": {
    "exclude": ["main.html"]
  }
}
```

This excludes indexing `main.html` directly, but does so through `overrides.json`. As the user interface uses the first title it sees, the title is overwritten.

#### Example: Overriding a Document's Link
Another example use case might be to redirect to another domain using the [`link` field](./fields.md#default-field-configuration):


```json
{
  "link": "https://infi-search.com",
  "_add_files": "./main.html"
}
```

> Overrides should be provided with JSON, CSV, or HTML files, as TXT and PDF files have no reliable way of supplying the `_add_files` field. In addition, you will need to manually map the CSV data to the `_add_files` field. This is automatically done for JSON and [HTML](../linking_to_others.md) files.


## Larger Collections

> ⚠️ This section is mostly for reference, use the preconfigured [scaling presets](../larger_collections.md) if possible.

```json
{
  "indexing_config": {
    "pl_limit": 4294967295,
    "pl_cache_threshold": 0,
    "num_pls_per_dir": 1000
  }
}
```

#### Index Shard Size: **`pl_limit`**

This is the main threshold parameter (in bytes) at which to "cut" index (**pl** meaning [postings list](https://en.wikipedia.org/wiki/Inverted_index)) files.

Increasing this value produces less but bigger files (which may take longer to retrieve), and vice versa.

Increasing the value may also be useful for caching when used in conjunction with `pl_cache_threshold` below, since fewer index files will be produced.

<br>

#### Index Caching: **`pl_cache_threshold`**

Index files that exceed this number will be cached by the search library at initilisation.

It can be used to configure InfiSearch for response time (over scalability) for some use cases. This is discussed in more detail in [Larger Collections](../larger_collections.md).
