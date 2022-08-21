# Larger Collections

*Five* configuration presets are available for scaling this tool to larger collections. They are designed primarily for Morsels' main intended use case of supporting static site search.

## Introduction

Each preset primarily makes a tradeoff between the **document collection size** it can support, the number of rounds of **network requests** (`RTT`) and the **number of files** generated (if file bloat is of concern). Morsels also supports generating result previews from [source files](../search_configuration.md#1-from-source-documents), if it is preferable to reduce file bloat from additional field stores.

The default preset is `small`, which generates a monolithic index and field store, much like other client side indexing tools.

Specify the `preset` key in your configuration file to change this.

```json
{
    "preset": "small" | "medium" | "large" | "medium_source" | "large_source"
}
```

## Presets

> `small`, `medium` and `large` corresponds to 0, 1, or 2 rounds of network requests in the table below.


| Preset              | Description |
| -----------         | ----------- |
| `small`             | Generates a monolithic index and field store. Identical to most other client side indexing tools.
| `medium`            | Generates a monolithic index but sharded (on a per document basis) field store. Only required field stores are retrieved for generating result previews. Positions are not indexed.
| `large`             | Generates both a sharded index and field store. Only index files that are required for the query are retrieved. Keeps [stop words](../language.md#stop-words). This is the preset used in the demo [here](https://morsels-search.com)!
| `medium_source`     | Generates a monolithic index and field store of source document links. Uses the links to retrieve source documents for result preview generation. Positions are not indexed.
| `large_source`      | Generates a sharded index and monolithic field store of source document links. Uses the links to retrieve source documents for result preview generation. Keeps [stop words](../language.md#stop-words).

#### Notes

- The 2 `large` presets do not remove stop words by default. This is because these options split up the index, which means that such commonly occuring words are likely to be separately placed into one file. (and never requested until necessary)
- The 2 `medium` presets do not index positions by default. Since positions take up a considerable proportion of the index size and a *sizeable* (at least, more so than `small`) monolithic index is assumed to be generated. The downside is that term proximity ranking and phrase queries will not be available.
- Note that scaling this tool for larger collections dosen't come freely, and necessitates fragmenting the index and/or field stores, **retrieving only what's needed**. This means extra network requests, but to a reasonable degree.

  This tool should be able to handle `800MB` (not counting things like HTML tags) collections with the full set of features enabled in the `large` preset.

## Modified Properties

Presets modify the following properties:

- Search Configuration: 
  - [`cacheAllFieldStores`](../search_configuration.md#search-functionality-options)
- Language Configuration:
  - [`ignore_stop_words`](../language.md#stop-words)
- Indexing Configuration:
  - What [fields](./fields.md) are stored (`do_store`). Not set if `fields_config.fields` is present.
  - [`field_store_block_size`](./fields.md)
  - [`pl_limit`](./indexing.md#indexing-and-search-scaling-advanced)
  - [`pl_cache_threshold`](./indexing.md#indexing-and-search-scaling-advanced)
  - [`with_positions`](indexing.md#miscellaneous-options)

Any of these values specified in the configuration file will override that of the preset's.


### Other Options

There are a few other options especially worth highlighting that can help reduce the index size (and hence support larger collections) or modify caching strategies.

- [`plLazyCacheThreshold`](../search_configuration.md#caching-options-advanced)

  In addition to **upfront** caching of index files with the `pl_cache_threshold` indexing parameter, Morsels also persistently caches any index shard that was requested before, but fell short of the `pl_cache_threshold`.
- [`ignore_stop_words=false`](../language.md#stop-words)

  This option is mostly only useful when using the `small / medium` presets which generate a monolithic index. Ignoring stop words in this case can reduce the overall index size.
