# Indexer Field Configuration

Every document you index contains multiple fields. By default, InfiSearch comes baked in with the configurations needed for supporting static site search.

## Default Field Configuration

It is helpful to first know of the default fields in InfiSearch and how they are used in the UI:

```json
{
  "fields_config": {
    "fields": {
      "title":        { "weight": 2.0, "k": 1.2, "b": 0.15 },
      "h1":           { "weight": 2.0, "k": 1.2, "b": 0.15 },
      "heading":      { "weight": 1.5, "k": 1.2, "b": 0.25 },
      "body":         { "weight": 1.0 },
      // The default weight is 0.0. These fields are stored, but not searchable.
      "headingLink":  {},
      "link":         {},
      "_relative_fp": {} // An internal, reserved field (see "Reserved Fields")
    }
  }
}
```

| Field       | Source                | UI Usage              |
| ----------- | -----------           | -----------           |
| h1, title | `<h1>`, `<title>` | Result preview's title. When unavailable, the `_relative_fp` field is displayed as a [breadcrumb](../search_configuration.md#general-options).
| heading | `<h2-6>` | Result preview sub match's heading.
| headingLink | `<h2-6 id="..">` | Result preview sub match's `#anchor` link.
| body | `<body>` | Result preview sub match's main text.
| _relative_fp | Relative file path from the source indexer folder | Result preview's `<a>` link by concatenating [`sourceFilesUrl`](../search_configuration.md#site-url) to `_relative_fp`
| link | User supplied override for [linking to other pages](../linking_to_others.md) | Result preview's `<a>` link. Convenience default field to support custom overrides for links easily (e.g. when indexing a JSON document).

<br>
<details>
<summary style="cursor: default;">Click to view Graphical Illustration</summary>
<img alt="annotation for fields" src="../images/fields_annotated.png" />
</details>


## Adding Fields

You can add your own fields to index as well, which will be factored into InfiSearch's search algorithms.

The user interface only incorporates the default set of fields to generate result previews however. If you need to incorporate additional fields, for example to display a icon beside each result, you can [alter](../search_configuration_renderers.md#rendering-search-results) the HTML outputs, or use the [search API](../search_api.md).

If don't need any of InfiSearch's default fields, you can also assign a value of `null` to remove it completely.

```json
{
  "fields_config": {
    "fields": {
      "h1": null
    }
  }
}
```

#### Reserved Fields

Reserved fields are prefixed with an underscore `_`, and are hardcoded into the indexer to perform special functions. You can still modify its field definition as desired (for example its `storage` parameter).

- **_relative_fp**: the relative path from your source folder to the file.

- **_add_files**: This field allows you to **index/combine multiple files** as **a single document**, which can be useful for overriding or extending data.
  See this [section](./misc.md#indexing-multiple-files-under-one-document) under indexing for more details.

## Field Specific Parameters

#### Field Scoring

**`weight=0.0`**

This parameter is a boost / penalty multiplied to a individual field's score.

Specifying `0.0` will also result in the field not being indexed into InfiSearch's inverted index at all meaning that searching for any terms in this field will not show up any results. The use case may be to create a field that is only stored for UI purposes (for example the `_relative_fp` field), when used in combination with the `storage` parameter.

**`k=1.2` & `b=0.75`**

These are Okapi BM25 model parameters that control the impact of term frequency and document lengths. The following [article](https://www.elastic.co/guide/en/elasticsearch/guide/current/pluggable-similarites.html#bm25-tunability) provides a good overview on how to configure these, if the defaults are unsuitable for your use case.

#### Field Storage: **`storage=["text"]`**

As with most information retrieval tools, InfiSearch performs full-text search using an [inverted index](https://en.wikipedia.org/wiki/Inverted_index) mapping terms to source documents.

Once the result set is obtained however, each result document's data could be useful for performing several operations. For example, a document's original **title** is essential for generating a human-readable result preview.

InfiSearch currently provides 2 storage formats, which can be used simultaneously in a single field:

**1. `text`**

In this format, raw texts of fields are stored into a JSON file as a series of `[fieldName, fieldText]` pairs as seen in the order in the document.

This "positioned" model is slightly more complex than a regular key-value store but enables the detailed content hierarchy you see in InfiSearch's UI currently: *Title > Heading > Text under Heading*

**2. `enum`**

This storage format stores a **single** value for each indexed document. Only the first such occurence will be stored, if there are multiple found. This is useful for data that is categorical in nature. These values can then be queried using the [search API](../search_api.md#filtering-enum-values) or used in the search UI to create [multi-select](../search_configuration.md#general-options) filters.

In this documentation for example (and the mdBook plugin), there is a multi-select checkbox filter that can be used to filter each page by it's mdBook section title. ("User Guide", "Advanced")

Notes:
- Documents without enum values are internally assigned a default enum value that can be queried.
- While it is unlikely you will need more, there is a hard limit of 255 possible values for your entire document collection. Values found in excess of this will be ignored, and the CLI indexer tool will print a warning.
- You can also use InfiSearch's flexible [boolean syntaxes](../search_syntax.md) to filter documents. Using this option instead however allows a simplifying assumption to store these values far more compactly.

<br>

**Configuring Field Storage for Larger Collections**

⚠️ This section is mostly for reference, consider using the preconfigured [scaling presets](../larger_collections.md) for scaling InfiSearch to larger collections.

```json
{
  "fields_config": {
    "cache_all_field_stores": true,
    "num_docs_per_store": 100000000
  }
}
```

**Field Store Caching: `cache_all_field_stores`**

This is the same option as the one under [search functionality options](../search_configuration.md#search-functionality-options).
If both are specified, the value specified in the `infisearch.init` takes priority.

All fields specified with `storage=["text"]` are cached up front on initialisation when this is enabled.

**Field Store Granularity: `num_docs_per_store`**

The `num_docs_per_store` parameter controls how many documents to store in one JSON file. Batching multiple files together if the fields stored are small can lead to less files and better browser caching.
