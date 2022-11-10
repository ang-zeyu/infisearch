# Filters

Multi-select filters, for example the ones you see in this documentation ("User Guide", "Advanced"), allow users to refine their searches.

To set these up, you will need to first consider your search domain. For example, a catalog of GPUs might have a "brand" category, while a catalog of weather forecast articles would each have an associated "weather". The common line is that you will need to provide problem-specific data.

For this guide, let's suppose we have a bunch of weather forecast articles and want to support filtering them by the weather (sunny, warm, cloudy).

First, we should setup a custom [field](./indexer/fields.md) inside your indexer configuration file.

```json
"fields_config": {
  "fields": [
    // Copy in the default fields,
    { "name": "title",        "weight": 2.0, "k": 1.2, "b": 0.15 },
    { "name": "h1",           "weight": 2.0, "k": 1.2, "b": 0.15 },
    { "name": "heading",      "weight": 1.5, "k": 1.2, "b": 0.25 },
    { "name": "body",         "weight": 1.0 },
    { "name": "headingLink",  "weight": 0.0 },
    { "name": "link",         "weight": 0.0 },
    { "name": "_relative_fp", "weight": 0.0 },

    // ----------------------------
    // Then add this field
    {
      "name": "weatherField",
      "storage": ["enum"],
      "weight": 0.0
    }
    // ----------------------------
  ]
}
```

The `is_enum: true` option tells Morsels that each document can only possibly contain one such value, allowing it to store these values far more efficiently than using a regular field. If there are multiple occurences, only the first seen value will be stored.

Next, we'll need to tell Morsels where the data for this field comes from.

For this guide, let's assume we're dealing with a bunch of HTML weather forecast articles in particular, which uses the [`HTMLLoader`](./indexer/indexing.md#html-files-loadershtmlloader). Our HTML files also store the weather inside a specific element with an `id="weather"`.

```json
"indexing_config": {
    "loaders": {
        "HtmlLoader": {
            "selectors": [
                // Copy in the default selectors,
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
                },
                {
                    "attr_map": {
                        "data-morsels-link": "link"
                    },
                    "selector": "span[data-morsels-link]"
                },
                // ----------------------------
                // Then add this field
                {
                    "attr_map": {},
                    "selector": "#weather",
                    "field_name": "weatherField" // matching our earlier defined field
                }
                // ----------------------------
            ]
        }
    }
}
```

Lastly, we need to tell Morsels' UI to setup a [multi-select](./search_configuration.md#general-options) filter using this field. To do so, add the following to your `initMorsels` call.

```ts
morsels.initMorsels({
    ...
    uiOptions: {
        multiSelectFilters: [
            {
                fieldName: 'weatherField', // matching our earlier defined field
                displayName: 'Weather',
                defaultOptName: 'Probably Sunny!'
            },
            // You can setup more filters as needed following the above procedures
        ]
    }
})
```

The `displayName` option tells the UI how to display the multi-select's header. We simply use an uppercased "Weather" in this case for readability.

Some of the weather forecast articles indexed may also be missing the `id="weather"` element, for example due to a bug in generating the article, and therefore lacks an enum value. Morsels internally assigns such documents a default enum value by default. The `defaultOptName` option specifies the name of this default enum value as seen in the UI.
