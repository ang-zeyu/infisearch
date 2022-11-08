# Field Configuration

Every document you index contains multiple fields. By default, Morsels comes baked in with the configurations needed for supporting static site search.

## Default Field Configuration

It may be helpful to first understand what the default fields are in Morsels, and how they are used in the user interface:

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


<img alt="annotation for fields" src="../images/fields_annotated.png" />

- **`h1`, `title`**: this is the header for a single document match, sourced from the HTML `<h1>` or `<title>` tags. If unavailable, the `_relative_fp` field is displayed as a breadcrumb (e.g. "user guide » introduction").

- **`heading`**: these are sourced from `<h2-6>` tags. It may contain corresponding highlights from **`body`** fields that are displayed below it.

  - **`headingLink`**: these are the corresponding `id` attributes of the heading tags. If available, an `#anchor` is appended to the document's link.

- **`_relative_fp`**: together with the provided `sourceFilesUrl` option, this field is for generating the link to the source document and (optionally).

- **`link`**: serves to support custom data requirements (e.g. linking to another page, indexing a json document), providing a means to override the default link of `sourceFilesUrl + _relative_fp`.

## Adding Fields

You can add your own fields to index as well, which will be factored into Morsels' search algorithms.

As explained in the default field configurations however, the user interface only incorporates the default set of fields to generate result previews (e.g. for term highlighting). If you need to incorporate additional fields, for example a link to an icon, you will need to [alter](../search_configuration_renderers.md#rendering-search-results) the HTML outputs, or use the [search API](../search_api.md).

If don't need any of Morsels' default fields, you can also assign a value of `null` to remove it completely.

```json
{
  "fields_config": {
    "fields": {
      "h1": null
    }
  }
}
```

## Reserved Fields

Reserved fields are prefixed with an underscore `_`, and are hardcoded into the indexer to perform special functions. You can still modify its field definition as desired (for example its `storage` parameter).

- **_relative_fp**: the relative path from your source folder to the file.

- **_add_files**: This field allows you to **index/combine multiple files** as **a single document**, which can be useful for overriding or extending data.

  See this [section](./indexing.md#indexing-multiple-files-under-one-document) under indexing for more details.

## Field Specific Parameters

#### Field Storage: **`storage=["text"]`**

As with most information retrieval tools, Morsels performs full-text search using an [inverted index](https://en.wikipedia.org/wiki/Inverted_index) mapping terms to source documents.

Once the result set is obtained however, each result document's data could be useful for performing several operations. For example, a document's original **title** is essential for generating a human-readable result preview.

Morsels currently provides 2 storage formats, which can be used simultaneously in a single field:

**1. `text`**

In this format, raw texts of fields are stored into a JSON file as a series of `[fieldName, fieldText]` pairs as seen in the order in the document.

This "positioned" model is slightly more complex than a regular key-value store but enables the detailed content hierarchy you see in Morsels' UI currently: *Title > Heading > Text under Heading*

**2. `enum`**

This storage format stores a **single** value for each indexed document. Only the first such occurence will be stored, if there are multiple found. In this documentation for example (and the mdBook plugin), there is a multi-select checkbox filter that can be used to filter each page by it's mdBook section title. ("User Guide", "Advanced")

This storage type should therefore be used for values that are "categorical" and finite in nature, and is useful for filtering documents by said categories.

You can also use Morsels' regular inverted index and flexible [boolean syntaxes](../search_features.md) to filter documents. Using this option instead however allows a simplifying assumption to store these values far more compactly. These values can then be queried using the [search API](../search_api.md#filtering-enum-values) or used in the search UI to create [multi-select](../search_configuration.md#general-options) filters.

 Documents that don't have any enum values will internally be assigned a default enum value that can also be queried. While it is highly unlikely that you will need more, note that there is also a hard limit of 255 possible values for your entire document collection. Values found in excess of this will be ignored, and the CLI indexer tool will print a warning.

#### Field Scoring Parameters

**`weight=0.0`**

This parameter is a boost / penalty multiplied to a individual field's score.

Specifying `0.0` will also result in the field not being indexed into Morsels' inverted index at all. Searching for any terms in this field will not show up any results. The use case may be to create a field that is only stored for UI purposes (for example the `_relative_fp` field), when used in combination with the `storage` parameter.

**`k=1.2` & `b=0.75`**

These are Okapi BM25 model parameters that control the impact of term frequency and document lengths. The following [article](https://www.elastic.co/guide/en/elasticsearch/guide/current/pluggable-similarites.html#bm25-tunability) provides a good overview on how to configure these, if the defaults are unsuitable for your use case.


## Larger Collections

> ⚠️ This part is mostly informational, and allows you to adjust caching strategies, and the number of generated files. However, you should mostly be well-served by the preconfigured [scaling presets](./larger_collections.md) for such purposes.

```json
{
  "fields_config": {
    "cache_all_field_stores": true,
    "num_docs_per_store": 100000000,
    "num_stores_per_dir": 1000,
  }
}
```

#### Field Store Granularity: **`num_docs_per_store`, `num_stores_per_dir`**

The `num_docs_per_store` parameter controls how many documents to store in one json file. Batching multiple files together if the fields stored are small can lead to less files and better browser caching. The `num_stores_per_dir` parameter controls how many json files should be stored together in one directory.

> ⚠️ Ensure `num_docs_per_store` is a clean multiple or divisor of the `num_docs_per_block` parameter under [indexing](./indexing.md).<br>
> This is a rather arbitiary limitation chosen to reduce the field store indexing scheme complexity,
> but should work well enough for most use cases.

#### Field Store Caching: **`cache_all_field_stores`**

This is the same option as the one under [search functionality options](../search_configuration.md#search-functionality-options).
If both are specified, the value specified in the `morsels.initMorsels` call will take priority.

All fields specified with `storage=["text"]` would be cached up front on initialisation of the search library.

Its usage alongside other options is discussed in more detail under the chapter [Larger Collections](larger_collections.md).
