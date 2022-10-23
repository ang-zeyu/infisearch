# Linking to other pages

Morsels is extremely easy to get started with if the pages you link to are the same files you index, and these files are hosted at the [`sourceFilesUrl`](./search_configuration.md#base-url) in the same manner your source file folders are structured.

Linking to other pages is facilitated by the default [`link`](./indexer/fields.md#default-field-configuration) field, which lets you override the link used in the result preview. You will however need to let Morsels know where to find the content for this field.

The section below covers only the common case for HTML files, which has some default configurations setup already. If you are using JSON, CSV, please refer to this [section](./indexer/indexing.md#indexing-multiple-files-under-one-document).

## Indexing HTML Files

If you are still indexing HTML files, you can simply add this link directly to some attribute of a hidden element.

For example,

```html
<span data-morsels-link="https://www.google.com"></span>
```

Then, modify your indexer [loader configuration](./indexer/indexing.md#html-files-loadershtmlloader) to let Morsels know to extract the `data-morsels-link` attribute of the `span[data-morsels-link]` into the `link` field.

This configuration is **already** implemented by default, and is attached here for reference.

```json
"loaders": {
  "HtmlLoader": {
    "exclude_selectors": [
      "script,style,pre"
    ],
    "selectors": [
      // Add the following object to the default configuration
      // If you've never configured "selectors" before, you will need to
      // add this entire snippet into your indexer configuration file.
      {
        "attr_map": {
          "data-morsels-link": "link"
        },
        "field_name": null,
        "selector": "span[data-morsels-link]"
      },
      {
        "attr_map": {},
        "field_name": "title",
        "selector": "title"
      },
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
          "id": "headingLink"
        },
        "field_name": "heading",
        "selector": "h2,h3,h4,h5,h6"
      }
    ]
  }
}
```
