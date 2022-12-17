# Linking to other pages

InfiSearch is convenient to get started with if the pages you link to are the same files you index, and these files are hosted at [`sourceFilesUrl`](./search_configuration.md#site-url) in the same way your source file folders are structured.

Linking to other pages instead is facilitated by the default [`link`](./indexer/fields.md#default-field-configuration) field, which lets you override the link used in the result preview.

There is also a default data mapping for HTML files which the below section covers. If using JSON or CSV files, refer to the earlier [section](./indexer/files.md).

## Indexing HTML Files

For HTML files, simply add this link with the `data-infisearch-link` attribute.

```html
<span data-infisearch-link="https://www.google.com"></span>
```

This data mapping configuration is **already** implemented by default, shown by the below snippet.

```json
"loaders": {
  "HtmlLoader": {
    "selectors": {
      "span[data-infisearch-link]": {
        "attr_map": {
          "data-infisearch-link": "link"
        }
      }
    }
  }
}
```
