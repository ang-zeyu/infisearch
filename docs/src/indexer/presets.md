# Larger Collections

5 configuration presets are designed primarily for use with Morsels' main intended use case of supporting static site search.

Each preset primarily makes a tradeoff between the **document collection size** it can support, and the number of rounds of **network requests** (`RTT`). Morsels also supports generating result previews from [source files](../search_configuration.md#1-from-source-documents), if it is preferable (e.g. to reduce file bloat from additional field stores).

The default preset is `small`, which generates a monolithic index and field store, much like other client side indexing tools.

Specify the `preset` key in your configuration file to change this.

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
| `large`             | Generates both a sharded index and field store. Only index files that are required for the query are retrieved. Keeps [stop words](./language.md#stop-words). This is the preset used in the demo [here](https://ang-zeyu.github.io/morsels-demo-1/)!
| `medium_source`     | Generates a monolithic index and field store of source document links. Uses the links to retrieve source documents for result preview generation.
| `large_source`      | Generates a sharded index and monolithic field store of source document links. Uses the links to retrieve source documents for result preview generation. Keeps [stop words](./language.md#stop-words).

> The 2 `large` presets do not remove stop words by default. This is because these options split up the index, which means that such commonly occuring words are likely to be separately placed into one file. (and never requested until necessary)

## Modified Properties

Presets modify the following properties:

- Search Configuration: 
  - [`cacheAllFieldStores`](../search_configuration.md#search-functionality-options)
- Language Configuration:
  - [`ignore_stop_words`](./language.md#stop-words)
- Indexing Configuration:
  - What [fields](./fields.md) are stored (`do_store`). Not set if `fields_config.fields` is present.
  - [`field_store_block_size`](./fields.md)
  - [`pl_limit`](./indexing.md#indexing-and-search-scaling-advanced)
  - [`pl_cache_threshold`](./indexing.md#indexing-and-search-scaling-advanced)

Any of these values specified in the configuration file will override that of the preset's.


### Other Options

There are 2 other options especially worth highlighting that can help reduce the index size (and hence support larger collections) in general.

- [`ignore_stop_words=false`](language.md#note-on-stop-words)
- [`with_positions=false`](indexing.md#miscellaneous-options)<br>
  Positional information takes up a considerable (up to **3-4** times larger) proportion of the index size!

If you are willing to forgo some features (e.g. phrase queries, boolean queries of stop words) in return for reducing the index size, you can enable / disable these options as appropriate.
