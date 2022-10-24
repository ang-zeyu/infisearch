# Indexing Configuration

The configurations in this section mainly specify **how** (mapping file contents to fields) and **which** files to index.

All configurations are optional, except `loaders`. The CLI tool will do nothing if the `loaders` dictionary is empty.

## Mapping File Data to Fields

```json
{
  "indexing_config": {
    "loaders": {
      // Only HTML files are indexed by default
      "HtmlLoader": {}
    }
  }
}
```

The indexer is able to handle data from HTML, JSON, CSV, TXT, or PDF files. Support for each file type is provided by a file **"Loader"** abstraction.

You may configure loaders by including them under the **`loaders` key**, with any applicable options.


#### HTML Files: **`loaders.HtmlLoader`**

```json
"loaders": {
  "HtmlLoader": {
    "exclude_selectors": [
      // Selectors to exclude from indexing
      "script,style,form,nav,[data-morsels-ignore]"
    ],
    "selectors": [
      {
        "attr_map": {},
        "field_name": "title",
        "selector": "title"
      },
      // <h1> tags are indexed into a separate field,
      // and has priority over the title in the generated SERP.
      {
        "attr_map": {},
        "field_name": "h1",
        "selector": "h1"
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
        "selector": "h2,h3,h4,h5,h6"
      },
      // Provides a means to override the link used in the result preview
      // See "Linking to other pages" for more information
      {
        "attr_map": {
          "data-morsels-link": "link"
        },
        "field_name": null,
        "selector": "span[data-morsels-link]"
      },
    ]
  }
}
```

1. The HTML loader traverses the document depth-first, in the order text nodes and attributes appear.

2. At each element, it checks if any selectors under `selectors.selector` matches the element. If so, all descendants (elements, text) of that element will be indexed under the specified `field_name`, if any.

   - This process repeats as the document is traversed — if a descendant matched another different selector, the field mapping is overwritten for that descendant and its descendants.

   - The `attr_map` option allows indexing attributes of specific elements under fields as well.

To **exclude elements** from indexing, you can use the `exclude_selectors` option, or add the in-built `data-morsels-ignore` attribute to your HTML.

If needed, you can also index **HTML fragments** that are incomplete documents. To match the entire fragment, use the `body` selector.

#### JSON Files: **`loaders.JsonLoader`**

```json
"loaders": {
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

JSON files can also be indexed. The `field_map` must be specified, which contains a mapping of **JSON data key -> field name**.
The `field_order` array controls the order in which these fields are indexed, which has a minor influence on [query term proximity ranking](../search_features.md#ranking-model).

The JSON file can be either:
1. An object, following the schema set out in `field_map`
2. An array of objects following the schema set out in `field_map`


#### CSV Files: **`loaders.CsvLoader`**

```json
"loaders": {
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

#### PDF Files: **`loaders.PdfLoader`**

```json
"loaders": {
  "PdfLoader": {
    "field": "body",
  }
}
```

This loader indexes all content into a single field "body" by default.

The search result title would appear as `<...PDF file path breadcrumb...> (PDF)`, and when clicked upon will open the PDF in the browser.

#### Text Files: **`loaders.TxtLoader`**

```json
"loaders": {
  "TxtLoader": {
    "field": "field_name",
  }
}
```

This loader simply reads `.txt` files and indexes all its contents into a single field. This is not particularly useful without the `_add_files` feature [below](#indexing-multiple-files-under-one-document).

## Miscellaneous Options

```json
{
  "indexing_config": {
    "exclude": [
      "morsels_config.json"
    ],
    "include": [],

    "with_positions": true
  }
}
```

#### File Exclusions: **`exclude = ["morsels_config.json"]`**

Global file exclusions can be specified in this parameter, which is simply an array of file globs.

#### File Inclusions: **`include = []`**

Similarly, you can specify only specific files to index. This is an empty array by default, which indexes everything.

If a file matches both an `exclude` and `include` pattern, the `exclude` pattern will have priority.

#### Adding Positions: **`with_positions = true`**

This option controls whether positions will be stored.

Features such as phrase queries that require positional information will not work if this is disabled.

Turning this off for very large collections (~> 1GB) can increase the tool's scalability, at the cost of such features.

## Indexing Multiple Files Under One Document

You can index **multiple files** into **one document** using the reserved field [`_add_files`](./fields.md#reserved-fields). This can be particularly useful for overriding data on a case-by-case basis.

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
  "link": "https://morsels-search.com",
  "_add_files": "./main.html"
}
```

> Overrides should be provided with JSON, CSV, or HTML files, as TXT and PDF files have no reliable way of supplying the `_add_files` field. In addition, you will need to manually map the CSV data to the `_add_files` field. This is automatically done for JSON and [HTML](../linking_to_others.md) files.

## Indexer Performance

```json
{
  "indexing_config": {
    "num_threads": <number of physical cpus> - 1,
    "num_docs_per_block": 1000
  }
}
```

#### Number of Threads: **`num_threads`**

This is the number of threads to use, excluding the main thread. When unspecified, this is `max(min(num physical cores, num logical cores) - 1, 1)`.

#### Memory Usage: **`num_docs_per_block`**

> ⚠️ The parameters below this point allow you to adjust caching strategies, and the number of generated files. However, you should mostly be well-served by the preconfigured [scaling presets](./larger_collections.md) for such purposes.

This parameter roughly controls the memory usage of the indexer; You may think of it as "how many documents to keep in memory before flushing results".

If your documents are very small, increasing this *may* help improve indexing performance.

⚠️ Also ensure [`num_docs_per_store`](./fields.md#field-store-granularity-num_docs_per_store-num_stores_per_dir) is a clean multiple or divisor of this parameter.

## Indexing and Search Scaling (advanced)

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

It can be used to configure morsels for response time (over scalability) for some use cases. This is discussed in more detail in [Larger Collections](./larger_collections.md).

#### Index Shards per Directory: **`num_pls_per_dir`**

This parameter simply controls how many index files you want to store in a single directory.

While the default value should serve sufficiently for most use cases, some file systems are less efficient at handling large amounts of files in one directory. Tuning this parameter may help to improve performance when looking up a particular index file.
