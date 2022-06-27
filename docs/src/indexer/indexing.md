# Indexing Configuration

The configurations in this section mainly specify **how (mapping file contents to fields)** and **which** files to index.

All configurations are optional, save for the `loader_configs` key. The cli tool will *do nothing* if the `loader_configs` dictionary is empty.

The snippet below shows the default values:

```json
{
  "indexing_config": {
    "num_threads": <number of physical cpus> - 1,

    "num_docs_per_block": 1000,

    "exclude": [
      "morsels_config.json"
    ],

    "loader_configs": {
      "HtmlLoader": {}
    },
    
    "pl_limit": 4294967295,

    "pl_cache_threshold": 0,

    "num_pls_per_dir": 1000,

    "with_positions": false
  }
}
```


## Mapping File Data to Fields

The indexer is able to handle data from HTML, JSON, CSV, TXT, or PDF files. Support for each file type is provided by a "Loader" abstraction.

You may configure loaders by **including them under the `loader_configs` key**, with any applicable options.


#### HTML Files: **`loader_configs.HtmlLoader`**

```json
"loader_configs": {
  "HtmlLoader": {
    // list of selectors to exclude from indexing
    "exclude_selectors": [
      "script,style,pre"
    ],
    "selectors": [
      {
        "attr_map": {},
        "field_name": "title",
        "selector": "title"
      },
      {
        "attr_map": {},
        "field_name": "body",
        "selector": "body"
      },
      {
        "attr_map": {
          "id": "headingLink" // "store the id attribute under headingLink"
        },
        "field_name": "heading",
        "selector": "h1,h2,h3,h4,h5,h6"
      }
    ]
  }
}
```

The HTML loader traverses the document depth-first, in the order text nodes and attributes appear.

At each element, it checks if any of the selectors under the `selectors.selector` key matches the element. If so, all descendants (elements, text) of that element will then be indexed under the specified `field_name`, if any.

This process repeats as the document is traversed — if another of the element's descendants matched a different selector, the field mapping is overwritten for that descendant's descendants.

The `attr_map` option allows indexing attributes of specific elements under fields as well.

> If needed, you can also index HTML fragments. To match the entire fragment, use the `body` selector.

#### JSON Files: **`loader_configs.JsonLoader`**

```json
"loader_configs": {
  "JsonLoader": {
    "field_map": {
      "chapter_text": "body",
      "book_link": "link",
      "chapter_title": "title"
    },
    // Order in which to index the keys of the json {} document
    "field_order": [
      "book_link",
      "chapter_title",
      "chapter_text"
    ]
  }
}
```

Json files can also be indexed. The `field_map` key must be specified, which contains a mapping of **json key -> field name**.
The `field_order` array controls the order in which these fields are indexed, which can have a minor influence on [query term proximity ranking](../search_features.md#ranking-specifics), if positions are indexed.

The json file can be either:
1. An object, following the schema set out in `field_map`
2. An array of objects following the schema set out in `field_map`


#### CSV Files: **`loader_configs.CsvLoader`**

```json
"loader_configs": {
  "CsvLoader": {
    // ---------------------
    // Map data using csv headers
    "use_headers": false,
    "header_field_map": {},
    "header_field_order": [],
    // ---------------------
    // Or simply csv header indices
    "index_field_map": {
      "0": "link",
      "1": "title",
      "2": "body",
      "4": "heading"
    },
    "index_field_order": [
      1,
      4,
      2,
      0
    ],
    // ---------------------
    // Options for csv parsing, from the Rust "csv" crate
    "parse_options": {
      "comment": null,
      "delimiter": 44,
      "double_quote": true,
      "escape": null,
      "has_headers": true,
      "quote": 34
    }
  }
}
```

Field mappings for CSV files can be configured using one of the `field_map / field_order` key pairs. The `use_headers` parameter specifies which of the two pairs of settings to use.

The `parse_options` key specifies options for parsing the csv file. In particular, note that the `has_headers` key is distinct from and does not influence the `use_headers` parameter.

#### PDF Files: **`loader_configs.PdfLoader`**

```json
"loader_configs": {
  "PdfLoader": {
    "field": "body",
  }
}
```

This loader indexes all content into a single field "body" by default.

The search result title would appear as `<...pdf file path breadcrumb...> (PDF)`, and when clicked upon will open the pdf in the browser.

#### Text Files: **`loader_configs.TxtLoader`**

```json
"loader_configs": {
  "TxtLoader": {
    "field": "field_name",
  }
}
```

This loader simply reads `.txt` files and indexes all its contents into a single field.


## Miscellaneous Options

#### File Exclusions: **`exclude = ["morsels_config.json"]`**

Global file exclusions can be specified in this parameter, which is simply an array of file globs.

#### Adding Positions: **`with_positions = false`**

This option controls whether positions will be stored.

Features such as phrase queries that require positional information will not work if this is disabled.

Turning this off for very large collections (~> 1GB) can increase the tool's scalability, at the cost of such features.

## Indexing and Search Scaling

Prefer the in-built [scaling presets](./presets.md) option for configuring the tool's scalability. Where needed, the following options are available for finer control.

#### Index Shard Size: **`pl_limit`**

This is the main threshold parameter (in bytes) at which to "cut" index (**pl** meaning [postings list](https://en.wikipedia.org/wiki/Inverted_index)) files.

Increasing this value produces less but bigger files (which may take longer to retrieve), and vice versa.

Increasing the value may also be useful for caching when used in conjunction with `pl_cache_threshold` below, since fewer index files will be produced.

<br>

#### Index Caching: **`pl_cache_threshold`**

Index files that exceed this number will be cached by the search library at initilisation.

It can be used to configure morsels for response time (over scalability) for some use cases. This is discussed in more detail in [Tradeoffs](../tradeoffs.md).

#### Index Shards per Directory: **`num_pls_per_dir`**

This parameter simply controls how many index files you want to store in a single directory.

While the default value should serve sufficiently for most use cases, some file systems are less efficient at handling large amounts of files in one directory. Tuning this parameter may help to improve performance when looking up a particular index file.

## Indexing Performance

#### Number of Threads: **`num_threads`**

This is the number of threads to use, excluding the main thread. When unspecified, this is `max(min(num physical cores, num logical cores) - 1, 1)`.

#### Memory Usage: **`num_docs_per_block`**

This parameter roughly controls the memory usage of the indexer; You may think of it as "how many documents to keep in memory before flushing results".

If your documents are very small, increasing this *may* help improve indexing performance.

> ⚠️ Ensure [`field_store_block_size`](./fields.md) is a clean multiple or divisor of this parameter.
