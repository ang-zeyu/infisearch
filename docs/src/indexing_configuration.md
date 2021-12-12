# Indexer Configuration

All indexer configurations are sourced from a json file. By default, the cli tool looks for `morsels_config.json` in the source folder (first argument specified in the command).

You can run the cli command with the `--init` option to initialise the default configuration file in the source folder.

## `fields_config`

The first step to indexing any documents is defining the field configurations.

The default configurations are as follows, already setup for interfacing with the `@morsels/search-ui` package.

```json
{
  "fields_config": {
    "field_store_block_size": 250,
    "fields": [
      {
        "name": "title",
        "do_store": false,
        "weight": 0.2,
        "k": 1.2,
        "b": 0.25
      },
      {
        "name": "heading",
        "do_store": false,
        "weight": 0.3,
        "k": 1.2,
        "b": 0.3
      },
      {
        "name": "body",
        "do_store": false,
        "weight": 0.5,
        "k": 1.2,
        "b": 0.75
      },
      {
        "name": "headingLink",
        "do_store": false,
        "weight": 0.0,
        "k": 1.2,
        "b": 0.75
      },
      // Internal, hardcoded field sourced from the
      // relative file path of the file from the root directory
      //
      // Required for the default search ui preview generation methods
      // (see "Generating Result Previews" under search configuration)
      //
      // Nonetheless, if omitted, the field will not be stored.
      {
        "name": "_relative_fp",
        "do_store": true,
        "weight": 0.0,
        "k": 1.2,
        "b": 0.75
      }
    ]
  }
}
```

**`field_store_block_size` and `do_store`**

Morsels stores fields that have `do_store: true` specified in the field configuration into a json file in the output folder.

At search time, these fields saved in this manner from the json files are retrieved as-is.

The `field_store_block_size` parameter controls how many documents to store in one such json file. Batching multiple files together if the fields stored are small can lead to less files and better browser caching.

> ⚠️ Ensure `field_store_block_size` is a clean multiple or divisor of the `num_docs_per_block` parameter elaborated further below.<br>
> This is a rather arbitiary limitation chosen to reduce the field store indexing scheme complexity,
> but should work well enough for most use cases.

**`weight`**

This parameter simply specifies the weight the field should have during scoring.

Specifying `0.0` will result in the field not being indexed (although, it can still be stored for retrieval using `do_store`).

**`k` & `b`**

These are Okapi BM25 model parameters. The following [article](https://www.elastic.co/guide/en/elasticsearch/guide/current/pluggable-similarites.html#bm25-tunability) provides a good overview on how to configure these, although, the defaults should serve sufficiently.

<div style="display: none;">

**`type` (WIP)**

The only available types are `string` and `u32`.

This only affects how the fields are stored when the `do_store` parameter is specified (but not the indexing process).

`string` fields are stored in the manner illustrated above.

`u32` fields however are stored monolithically in a single file, for the purpose of fast random access.

Moreover, sorting (also WIP) operations are only supported on `u32` fields.
</div>

**`_relative_fp`**

This is a "hardcoded" field, in that its value is fixed as the relative file path from your source folder path to the file.

It is included in the default configuration to allow `@morsels/search-ui` to retrieve the source file for result preview generation. You may refer back to [this section](./search_configuration.md#options-for-generating-result-previews) for more details.

If this is removed, this field simply won't be indexed.

## `lang_config`

The snippet below shows the default values for language configuration. The key controlling the main tokenizer module to use is the `lang` key, while the `options` key supplies tokenization options unique to each module.

These options are also applied to `@morsels/search-ui`, which sources this information from the index output directory as specified in the `initMorsels` call.

```json
{
  "lang_config": {
    "lang": "latin",
    "options": null
  }
}
```

### Forenote on Stop Words

A slightly different approach with stop words is taken in that stop words are only filtered at **query time** for certain types of queries (currently this is for free-text queries with more than two terms).

This is because splitting up the index means that we are already able to put each of such commonly occuring words into one file, so, information for stop words is never requested unless necessary:
- For processing phrase queries (eg. `"for tomorrow"`)
- Boolean queries (eg. `if AND forecast AND sunny`)
- One or two term free text queries containing stop words only. This is an unlikely use case, but it is nice having some results show up than none.

### Ascii Tokenizer

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

    "max_term_len": 80
  }
}
```

### Latin Tokenizer

This is essentially the same as the ascii tokenizer, but adds a `stemmer` option.

```
"lang_config": {
  "lang": "latin",
  "options": {
    // Ascii Tokenizer options also apply

    // Any of the languages here
    // https://docs.rs/rust-stemmers/1.2.0/rust_stemmers/enum.Algorithm.html
    // For example, "english"
    "stemmer": "english"
  }
}
```

It is separated from the ascii tokenizer to reduce binary size (about ~`220KB` savings before gzip).

### Chinese Tokenizer

A basic `chinese` tokenizer based on [jieba-rs](https://github.com/messense/jieba-rs) is also available, although, it is still a heavy WIP at the moment. Use at your own discretion.

This tokenizer applies jieba's `cut` method to obtain various tokens, then applies a punctuation filter to these tokens. Thereafter, tokens are grouped into sentences.

```json
"lang_config": {
  "lang": "chinese",
  "options": {
    "stop_words": []
  }
}
```


### Remark on Language Modules' Flexibility

While using the same tokenizer for both indexing / search unifies the codebase, one downside is that code size has to be taken into account.

The chinese tokenizer for example, which uses *jieba-rs*, accounts for half of the wasm binary size alone.

Therefore, the tokenizers will aim to be reasonably powerful and configurable enough, such that the wasm bundle size dosen't blow up.

Nonetheless, if you feel that a certain configuration option should be supported for a given tokenizer but isn't, feel free to open up an issue!

## `indexing_config`

The configurations in this section specify **how and which** files to index.

All configurations are optional (reasonable defaults provided otherwise), save for the `loader_configs` key. The cli tool **will do nothing** if no loaders are specified.

The snippet below shows the default values, which need not be altered if you are only indexing html files.

```json
{
  "indexing_config": {
    // Number of threads excluding the main thread
    "num_threads": 5,      // when unspecified, this is max(num physical cores - 1, 1)

    // This roughly controls the memory usage of the indexer
    // If your documents are very small,
    // increasing this *may* help improve indexing performance.
    "num_docs_per_block": 1000,

    // glob patterns to exclude from indexing
    "exclude": [
      "morsels_config.json"
    ],

    // Specifies what types of files to index
    "loader_configs": {
      "HtmlLoader": {}     // enables support for .html files
    },
    
    // The threshold (in bytes) at which to "cut" index files
    //
    // Increasing this produces less but bigger files
    // (which may take longer to retrieve), and vice versa.
    
    "pl_limit": 16383,

    // Any index files above this size (in bytes)
    // will be pre-loaded on initialisation of the search library
    "pl_cache_threshold": 1048576,
    
    // Number of index files ("morsels") to store per directory
    "num_pls_per_dir": 1000,

    // Number of field stores (`.json` files) to store per directory
    "num_stores_per_dir": 1000,

    // Whether positions will be stored.
    //
    // Phrase queries and Query Term Proximity Ranking
    // will be unavailable if this is false.
    //
    // You may want to turn this off for very large collections. (~> 1GB)
    "with_positions": true
  }
}
```

### Loaders

The indexer is able to handle html, json or csv files. Support for each file type is provided by a "`Loader`" abstraction.

You may configure loaders by including them under the `loader_configs` key, with any applicable options.

The below sections shows the available loaders and configuration options available for each of them.

**`HtmlLoader`**

```json
{
  "HtmlLoader": {
    // list of selectors to exclude from indexing
    "exclude_selectors": [
      "script,style"
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

The html loader traverses the document depth-first.

At each element, it checks if any of the selectors under the `selectors.selector` key matches the element. If so, all descendants (elements, text) under this element will then be indexed under the field specified by `field_name`. If one the element's descendants matched another selector however, the configuration is then overwritten for that descendant (and its descendants).

The `attr_map` allows indexing attributes of elements (not including descendants) under fields as well.

**`JsonLoader`**

```json
{
  "JsonLoader": {
    "field_map": {
      "body": "body",
      "heading": "heading",
      "link": "_relative_fp",
      "title": "title"
    },
    // Order in which to index the fields of the json {} document
    "field_order": [
      "title",
      "heading",
      "body",
      "link"
    ]
  }
}
```

Json files can also be indexed. To do this, the `field_map` key must be specified, which contains a mapping of **json key -> field name**.
The `field_order` controls in which order these fields are indexed, which can influence position based functions such as query term proximity ranking.

The json files itself can be either
1. An object, following the schema set out in `field_map`
2. An array of objects following the schema set out in `field_map`

**`CsvLoader`**

```json
{
  "CsvLoader": {
    "use_headers": false,
    "header_field_map": {},
    "header_field_order": [],
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

Field mappings for csv can be configured using one of the `field_map / field_order` key pairs. The `use_headers` parameter specifies which of the two pairs of settings to use.




## Full Example

```json
{
  "fields_config": {
    "field_store_block_size": 250,
    "fields": [
      {
        "name": "title",
        "do_store": false,
        "weight": 0.2,
        "k": 1.2,
        "b": 0.25
      },
      {
        "name": "heading",
        "do_store": false,
        "weight": 0.3,
        "k": 1.2,
        "b": 0.3
      },
      {
        "name": "body",
        "do_store": false,
        "weight": 0.5,
        "k": 1.2,
        "b": 0.75
      },
      {
        "name": "headingLink",
        "do_store": false,
        "weight": 0.0,
        "k": 1.2,
        "b": 0.75
      },
      {
        "name": "_relative_fp",
        "do_store": true,
        "weight": 0.0,
        "k": 1.2,
        "b": 0.75
      }
    ]
  },
  "lang_config": {
    "lang": "latin",
    "options": null
  },
  "indexing_config": {
    "num_docs_per_block": 1000,
    "pl_limit": 16383,
    "pl_cache_threshold": 1048576,
    "exclude": [
      "morsels_config.json"
    ],
    "loader_configs": {
      "HtmlLoader": {
        "exclude_selectors": [
          ".no-index"
        ]
      },
      "JsonLoader": {
        "field_map": {
          "body": "body",
          "heading": "heading",
          "link": "_relative_fp",
          "title": "title"
        },
        "field_order": [
          "title",
          "heading",
          "body",
          "link"
        ]
      }
    },
    "pl_names_to_cache": [],
    "num_pls_per_dir": 1000,
    "num_stores_per_dir": 1000,
    "with_positions": true
  }
}
```


