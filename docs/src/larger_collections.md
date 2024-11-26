# Larger Collections

*Three* configuration presets are available for scaling this tool to larger collections. They are designed primarily for InfiSearch's main intended use case of supporting static site search.

## Introduction

Each preset primarily makes a tradeoff between the **document collection size** it can support and the number of rounds of **network requests** (`RTT`).

The default preset is `small`, which generates a monolithic index and field store, much like other client side indexing tools.

Specify the `preset` key in your configuration file to change this.

```json
{
    "preset": "small" | "medium" | "large"
}
```

## Presets

> `small`, `medium` and `large` corresponds to 0, 1, or 2 rounds of network requests in the table below.


| Preset              | Description |
| -----------         | ----------- |
| `small`             | Generates a monolithic index and field store. Identical to most other client side indexing tools.
| `medium`            | Generates an almost-monolithic index but sharded field store. Only required field stores are retrieved for generating result previews.
| `large`             | Generates both a sharded index and field store. Only index files that are required for the query are retrieved. Keeps [stop words](./language.md#stop-words). This is the preset used in the demo [here](https://ang-zeyu.github.io/infisearch-website)!

In summary, scaling this tool for larger collections dosen't come freely, and necessitates fragmenting the index and/or field stores, **retrieving only what's needed**. This means extra network requests, but to a reasonable degree.

  This tool should be able to handle `800MB` (not counting things like HTML tags) collections with the full set of features enabled in the `large` preset.

## Other Options

There are a few other options especially worth highlighting that can help reduce the index size (and hence support larger collections) or modify caching strategies.

- [`plLazyCacheThreshold`](./search_configuration.md#caching-options-advanced)

  In addition to **upfront** caching of index files with the `pl_cache_threshold` indexing parameter, InfiSearch also persistently caches any index shard that was requested before, but fell short of the `pl_cache_threshold`.
- [`ignore_stop_words=false`](./language.md#stop-words)

  This option is mostly only useful when using the `small / medium` presets which generate a monolithic index. Ignoring stop words in this case can reduce the overall index size, if you are willing to forgo its [benefits](./language.md#stop-words).
- [`with_positions=true`](./indexer/misc.md#indexing-positions)

  Positions take up a considerable (~3/4) portion of the index size but produces useful information for proximity ranking, and enables performing phrase queries.

## Modified Properties

Presets modify only the following properties:

- Search Configuration:  [`cacheAllFieldStores`](./search_configuration.md#search-functionality-options)
- Indexing Configuration: [`num_docs_per_store`](./indexer/misc.md#larger-collections), [`pl_limit`](./indexer/misc.md#larger-collections), [`pl_cache_threshold`](./indexer/misc.md#larger-collections)

Any of these values specified in the configuration file will override that of the preset's.

