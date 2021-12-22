# Indexer Configuration

All indexer configurations are sourced from a json file. By default, the cli tool looks for `morsels_config.json` in the source folder (first argument specified in the command).

You can run the cli command with the `--init` option to initialise the default configuration file in the source folder.


## Full Example

```json
{
  "fields_config": {
    "cache_all_field_stores": true,
    "field_store_block_size": 250,
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
      {
        "name": "_relative_fp",
        "do_store": true,
        "weight": 0.0,
        "k": 1.2,
        "b": 0.75
      }
    ]
  },
  "lang_config": {
    "lang": "latin",
    "options": null
  },
  "indexing_config": {
    "num_docs_per_block": 1000,
    "pl_limit": 16383,
    "pl_cache_threshold": 1048576,
    "exclude": [
      "morsels_config.json"
    ],
    "loader_configs": {
      "HtmlLoader": {
        "exclude_selectors": [
          ".no-index"
        ]
      },
      "JsonLoader": {
        "field_map": {
          "body": "body",
          "heading": "heading",
          "link": "_relative_fp",
          "title": "title"
        },
        "field_order": [
          "title",
          "heading",
          "body",
          "link"
        ]
      }
    },
    "pl_names_to_cache": [],
    "num_pls_per_dir": 1000,
    "with_positions": true
  }
}
```


