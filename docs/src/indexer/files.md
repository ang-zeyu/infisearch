# Indexer File Configuration

The configurations in this section mainly specify **how** (mapping file contents to fields) and **which** files to index.

InfiSearch's defaults should be sufficient to index most HTML files, but if not, you can also configure how the content mapping is done. Enabling support for other file formats (e.g. JSON, CSV, PDF) files is also done here.

## Mapping File Data to Fields

```json
{
  "indexing_config": {
    "loaders": {
      // Default: Only HTML files are indexed
      "HtmlLoader": {}
    }
  }
}
```

The indexer is able to handle data from HTML, JSON, CSV, TXT, or PDF files. Support for each file type is provided by a file *Loader* abstraction.

You may configure loaders by including them under the `loaders`, with any applicable options.


#### HTML Files: **`loaders.HtmlLoader`**

```json
"loaders": {
  "HtmlLoader": {
    "exclude_selectors": [
      // Selectors to exclude from indexing
      "script,style,form,nav,[data-infisearch-ignore]"
    ],
    "selectors": {
      "title": {
        "field_name": "title"
      },

      "h1": {
        // <h1> tags are indexed into a separate field,
        // and is used in the result preview over the title when available.
        "field_name": "h1"
      },

      "h2,h3,h4,h5,h6": {
        "attr_map": {
          // "store the id attribute under headingLink"
          "id": "headingLink"
        },
        "field_name": "heading"
      },

      "body": {
        "field_name": "body"
      },

      // Provides a means to override the link used in the result preview
      // See "Linking to other pages" for more information
      "span[data-infisearch-link]": {
        "attr_map": {
          "data-infisearch-link": "link"
        }
      }
    }
  }
}
```

The HTML loader indexes a document as such:

1. It traverses the document depth-first, in the order text naturally appears.

2. At each element, it checks if any selectors specified as keys under `HtmlLoader.selectors` is satisfied. If so, all descendants (elements, text) of that element are indexed under the newly specified `field_name`, if any.

   - This process repeats as the document is traversed — if a descendant matched another different selector, the field mapping is overwritten for that descendant and its descendants.

   - The `attr_map` option allows indexing attributes of specific elements under fields as well.

   - All selectors are matched in arbitrary order by default. To **specify an order**, add the `priority: n` key to your selector definition, where `n` is any integer.

To **exclude elements** from indexing, you can use the `exclude_selectors` option, or add the in-built `data-infisearch-ignore` attribute to your HTML.

If needed, you can also index **HTML fragments** that are incomplete documents. To match the entire fragment, use the `body` selector.

Lastly, if you need to remove a default selector, simply replace its definition with `null`. For example, `"h2,h3,h4,h5,h6": null`.

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
The `field_order` array controls the order in which these fields are indexed, which has a minor influence on [query term proximity ranking](../search_syntax.md#ranking-model).

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

This loader simply reads `.txt` files and indexes all its contents into a single field. This is not particularly useful without the `_add_files` [feature](#misc-multiple-files-under-one-document).

## File Exclusions

```json
{
  "indexing_config": {
    "exclude": [
      "infi_search.json"
    ],
    "include": [],

    "with_positions": true
  }
}
```

#### File Exclusions: **`exclude = ["infi_search.json"]`**

Global file exclusions can be specified in this parameter, which is simply an array of file globs.

#### File Inclusions: **`include = []`**

Similarly, you can specify only specific files to index. This is an empty array by default, which indexes everything.

If a file matches both an `exclude` and `include` pattern, the `exclude` pattern will have priority.