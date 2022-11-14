# Filters

Multi-select filters, for example the ones you see in this documentation ("User Guide", "Advanced"), allow users to refine their searches.

To set these up, you will need to first consider your search domain. For example, a catalog of GPUs might have a "brand" category, while a catalog of weather forecast articles would each have an associated "weather". The common line is that you will need to provide problem-specific data.

For this guide, let's suppose we have a bunch of weather forecast articles and want to support filtering them by the weather (sunny, warm, cloudy).

First, we should setup a custom [field](./indexer/fields.md) inside the indexer configuration file.

```json
"fields_config": {
  "fields": {
    "weatherField": {
      "storage": ["enum"]
    }
  }
}
```

The `is_enum: true` option tells InfiSearch that each document can only possibly contain one such value, allowing it to store these values far more efficiently than using a regular field. If there are multiple occurences, only the first seen value will be stored.

Next, we'll need to tell InfiSearch where the data for this field comes from.

For this guide, let's assume we're dealing with a bunch of HTML weather forecast articles in particular, which uses the [`HTMLLoader`](./indexer/indexing.md#html-files-loadershtmlloader). Our HTML files also store the weather inside a specific element with an `id="weather"`.

```json
"indexing_config": {
  "loaders": {
    "HtmlLoader": {
      "selectors": {
        "#weather": {
          // matching our earlier defined field
          "field_name": "weatherField"
        }
      }
    }
  }
}
```

Lastly, we need to tell InfiSearch's UI to setup a [multi-select](./search_configuration.md#general-options) filter using this field. To do so, add the following to your `init` call.

```ts
infisearch.init({
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

Some of the weather forecast articles indexed may also be missing the `id="weather"` element, for example due to a bug in generating the article, and therefore lacks an enum value. InfiSearch internally assigns such documents a default enum value by default. The `defaultOptName` option specifies the name of this default enum value as seen in the UI.
