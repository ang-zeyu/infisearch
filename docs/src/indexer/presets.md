# Presets

5 configuration presets are designed primarily for use with Morsels' main intended use case supporting static site search).

Each preset primarily makes a tradeoff between the document collection size it can support, and the number of rounds of network requests (`RTT`). Morsels also supports generating result previews from [source files](../search_configuration.md#1-from-source-documents), if it is preferable (e.g. to reduce file bloat from additional field stores).

The default preset is `small`, which generates a monolithic index and field store, much like other client side indexing tools. You may still want to use morsels since it packages a search UI, or, if you prefer the simplicity of a cli indexer tool (e.g. for CI build tools)

Specify the `preset` key in your configuration file to change the preset.

```json
{
    "preset": "small" | "medium" | "large" | "medium_source" | "large_source"
}
```

## Overview

> `small`, `medium` and `large` corresponds to 0, 1, or 2 rounds of network requests in the table below.


| Preset              | Description |
| -----------         | ----------- |
| `small`             | Generates a monolithic index and field store. Identical to most other client side indexing tools.
| `medium`            | Generates a monolithic index but sharded (on a per document basis) field store. Only field stores of documents to generate result previews for a retrieved.
| `large`             | Generates both a sharded index and field store. Only index files that are required for the query are retrieved. This is the preset used in the demo [here](https://ang-zeyu.github.io/morsels-demo-1/)!
| `medium_source`     | Generates a monolithic index and field store of source document links. Uses the links to retrieve source documents for result preview generation.
| `large_source`      | Generates a sharded index and monolithic field store of source document links. Uses the links to retrieve source documents for result preview generation.

## Modified Properties

Presets modify the following properties:

- Search Configuration: 
  - [`cacheAllFieldStores`](search_configuration.md#search-library-options)
- Indexing Configuration:
  - What [fields](./indexer/fields.md) are stored (`do_store`). Not set if `fields_config.fields` is present.
  - [`field_store_block_size`](./indexer/fields.md)
  - [`pl_limit`](./indexer/indexing.md#search-performance)
  - [`pl_cache_threshold`](./indexer/indexing.md#search-performance)

Any of these values specified in the configuration file will override that of the preset's.
