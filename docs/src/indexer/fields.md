# `fields_config`

The first step to indexing any documents is defining the fields to store.

The default configurations are as follows, already setup for interfacing with the `@morsels/search-ui` package.

If you are using morsels' search UI **as-is** (e.g. not adding additional fields to display), you can likely skip configuring `fields_config.fields`.

You may want to briefly take note of the other parameters under `fields_config` however, which can be used to adjust response times / file bloat. The possible adjustments are discussed later in [Tradeoffs](../tradeoffs.md).

```json
{
  "fields_config": {
    "cache_all_field_stores": true,
    "field_store_block_size": 10000,
    "num_stores_per_dir": 1000,
    "fields": [
      {
        "name": "title",
        "do_store": false,
        "weight": 0.2,
        "k": 1.2,
        "b": 0.25
      },
      {
        "name": "heading",
        "do_store": false,
        "weight": 0.3,
        "k": 1.2,
        "b": 0.3
      },
      {
        "name": "body",
        "do_store": false,
        "weight": 0.5,
        "k": 1.2,
        "b": 0.75
      },
      {
        "name": "headingLink",
        "do_store": false,
        "weight": 0.0,
        "k": 1.2,
        "b": 0.75
      },
      // Internal, hardcoded field (see "Special Fields")
      {
        "name": "_relative_fp",
        "do_store": true,
        "weight": 0.0,
        "k": 1.2,
        "b": 0.75
      }
    ]
  }
}
```

**`field_store_block_size`, `num_stores_per_dir`, and `do_store`**

Morsels stores fields that have `do_store: true` specified in the field configuration into a json file in the output folder.

At search time, the fields saved in this manner from the json files are retrieved for result preview generation.

The `field_store_block_size` parameter controls how many documents to store in one such json file. Batching multiple files together if the fields stored are small can lead to less files and better browser caching. The `num_stores_per_dir` parameter controls how many json files should be stored together in one directory.

> ⚠️ Ensure `field_store_block_size` is a clean multiple or divisor of the `num_docs_per_block` parameter elaborated under [indexing](./indexing.md) later.<br>
> This is a rather arbitiary limitation chosen to reduce the field store indexing scheme complexity,
> but should work well enough for most use cases.

**`cache_all_field_stores`**

This is the same as the configuration option under [search configuration](../search_configuration.md#search-library-options).
If both are specified, the value specified in the `initMorsels` call will take priority.

All fields specified with `do_store=true` would be cached up front on initialisation of the search library.

Its usage alongside other options is discussed in more detail under the chapter [Tradeoffs](../tradeoffs.md).

**`weight`**

This parameter simply specifies the weight the field should have during scoring.

Specifying `0.0` will result in the field not being indexed (although, it can still be stored for retrieval using `do_store`).

**`k` & `b`**

These are Okapi BM25 model parameters. The following [article](https://www.elastic.co/guide/en/elasticsearch/guide/current/pluggable-similarites.html#bm25-tunability) provides a good overview on how to configure these if the defaults are unsuitable for your use case.

## Default Fields in `@morsels/search-ui`

The functions of the default fields for the user interface are as follows:

<img alt="annotation for fields" src="../images/fields_annotated.png" />

- `title`: This is the header for a single document match. 
- `heading`: These are section headers which appear on the left of corresponding `body` fields. THey are sourced from `<h1-6>` tags by default.
- `headingLink`: These are the `id` attributes of corresponding `<h1-6>` tags. If available, an `#anchor` is attached to the linked document for the particular heading
- `body`: This field is the text that appears to the right of headings (or on its own if there is no corresponding heading).
- `_relative_fp` **or** `link`: If the `title` field is missing for any document, this field takes its place in the header. It is also used to link to the source document (in the `<a />` tag) or for generating result previews (more [here](../search_configuration.md#default-rendering-output--purpose)).
  - Note: The `link` field is not setup by default; The combination of `sourceFilesUrl` + `_relative_fp` serves the same purpose. The `link` field serves to accomodate more custom use cases (e.g. linking to another site, or linking to a HTML page by indexing a json document).



## Mapping File Data to Fields

Defining fields is all good, but you may also need a way to map custom-formatted file data to each of these fields if the default mappings are insufficient. This is discussed later under [indexing](./indexing.md#mapping-file-data-to-fields-loader_configs).

The exception are "special" fields below, which sources data from elsewhere.

### Special Fields

**`_relative_fp`**

This is a "hardcoded" field generated by the indexer, in that its value is fixed as the relative file path from your source folder path to the file.

It is included in the default configuration to allow `@morsels/search-ui` to retrieve the source file for result preview generation, and to link to the document itself (via an `<a></a>` tag). You may refer back to [this section](../search_configuration.md#options-for-generating-result-previews) for more details.

If this is removed, this field simply won't be indexed nor stored.
