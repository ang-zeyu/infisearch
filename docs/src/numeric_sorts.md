# Sorting by Numbers & Dates

Results can also be sorted by [numeric fields](./indexer/fields.md#field-storage). Let's suppose we want to support filtering weather forecast articles by their date posted. The date is stored in an element with the `data-date-posted` attribute.

First, we define the numeric [field](./indexer/fields.md#field-storage) that can store any signed 64-bit integers.

```json
"fields_config": {
  "fields": {
    "datePostedField": {
      "storage": [{
        "type": "i64",

        // Default UNIX timestamp.
        // In this case, we use "0", which falls on Jan 1 1970 00:00 UTC.
        "default": 0,

        // Parse the data seen as a date.
        // Integers, floats, and other datetime formats are also supported,
        // see the above linked documentation.
        "parse": {
          "method": "datetime",
          "datetime_fmt": "%Y %b %d %H:%M %z"
        }
      }]
    }
  }
}
```

Next, we map the data from the `data-date-posted` attribute into the above field.

```json
"indexing_config": {
  "loaders": {
    "HtmlLoader": {
      "selectors": {
        // Match elements with the attribute
        "[data-date-posted]": {
          // And index the attribute into our earlier defined field
          "attr-map": {
            "[data-date-posted]": "datePostedField"
          }
        }
      }
    }
  }
}
```

Lastly, we tell InfiSearch's UI to [setup the UI](./search_configuration.md#setting-up-numeric-filters-and-sort-orders) `<select>` element using this field. To do so, add the following to your `infisearch.init` call.

```ts
infisearch.init({
  ...
  uiOptions: {
    sortFields: {
      dateposted: {
        asc: 'Date: Oldest First',
        desc: 'Date: Latest First',
      },
    },
  }
})
```
