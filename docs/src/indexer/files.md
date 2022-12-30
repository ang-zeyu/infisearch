# Indexer Data Configuration

The configurations in this page specify **how** (mapping file data to fields) and **which** files to index.

InfiSearch's defaults are sufficient to index most HTML files, but if not, you can also configure how the content mapping is done. Enabling support for other file formats (e.g. JSON, CSV, PDF) files is also done here.

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

The `HTMLLoader` is the only loader that is configured by default, which is as follows:

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
        "field_name": "h1"
      },

      "h2,h3,h4,h5,h6": {
        "attr_map": {
          "id": "headingLink" // stores the id attribute under the headingLink field
        },
        "field_name": "heading"
      },

      "body": {
        "field_name": "body"
      },

      "meta[name=\"description\"],meta[name=\"keywords\"]": {
        "attr_map": {
          "content": "body"
        }
      },

      // A convenient means to override the link used in the result preview
      // See "Linking to other pages" for more information
      "span[data-infisearch-link]": {
        "attr_map": {
          "data-infisearch-link": "link"
        }
      }
    },
    "merge_default_selectors": true
  }
}
```

The HTML loader indexes a document by:

1. Traversing the document depth-first, in the order text naturally appears.

2. Checking if any selectors specified as keys under `HtmlLoader.selectors` is satisfied for each element. If so, all descendants (elements, text) of the element are indexed under the newly specified `field_name`, if any.

   - This process repeats as the document is traversed — if a descendant matched another different selector, the field mapping is overwritten for that descendant and its descendants.

   - The `attr_map` option allows indexing attributes of specific elements under fields as well.

   - All selectors are matched in arbitrary order by default. To **specify an order**, add a higher `priority: n` key to your selector definition, where `n` is any integer.

To **exclude elements** from indexing, use the `exclude_selectors` option, or add the in-built `data-infisearch-ignore` attribute to your HTML.

If needed, you can also index **HTML fragments** that are incomplete documents (for example, documents which are missing the `<head>`). To match the entire fragment, use the `body` selector.

Lastly, if you need to remove a default selector, simply replace its definition with `null`. For example, `"h2,h3,h4,h5,h6": null`. Alternatively, specifying `"merge_default_selectors": false` will remove all default selectors.

#### JSON Files: **`loaders.JsonLoader`**

```json
"loaders": {
  "JsonLoader": {
    "field_map": {
      "chapter_text": "body",
      "book_link": "link",
      "chapter_title": "title"
    },
    // Optional, order in which to index the keys of the json {} document
    "field_order": [
      "book_link",
      "chapter_title",
      "chapter_text"
    ]
  }
}
```

JSON files can also be indexed. The `field_map` contains a mapping of your **JSON data key -> field name**.
The `field_order` array controls the order in which the data keys are indexed, which has a minor influence on [query term proximity ranking](../introduction.md#ranking-model).

The JSON file can be either:
1. An object, with numbers, strings or `null` values
2. An array of such objects


#### CSV Files: **`loaders.CsvLoader`**

```json
"loaders": {
  "CsvLoader": {
    // ---------------------
    // Map data using CSV headers
    "header_field_map": {},
    "header_field_order": [],            // Optional, order to index the columns
    // ---------------------
    // Or with header indices
    "index_field_map": {
      "0": "link",
      "1": "title",
      "2": "body",
      "4": "heading"
    },
    "index_field_order": [1, 4, 2, 0],   // Optional, order to index the columns
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

Field mappings for CSV files can be configured using one of the `field_map` keys. The `field_order` arrays controls the order columns are indexed.

The `parse_options` key specifies options for parsing the csv file.

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

This loader simply reads `.txt` files and indexes all its contents into a single field. This is not particularly useful without the `_add_files` [feature](./misc.md#indexing-multiple-files-under-one-document) feature that allows indexing data from multiple files as one document.

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
