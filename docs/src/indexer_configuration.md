# Indexer Configuration

All indexer configurations are sourced from a json file. By default, the cli tool looks for `morsels_config.json` in the source folder (first argument specified in the command).

You can run the cli command with the `--config-init` option to initialise the full, default configuration file in the source folder. As the file generated from this option is rather verbose, you could also instead override the necessary properties as covered in the subsequent sections.


## Full Example

A typical full configuration file looks something like this:

```json
{
  "preset": "small",
  "fields_config": {
    "field_store_block_size": 4294967295,
    "num_stores_per_dir": 1000,
    "cache_all_field_stores": true,
    "fields": [
      {
        "name": "title",
        "do_store": true,
        "weight": 0.2,
        "k": 1.2,
        "b": 0.25
      },
      {
        "name": "heading",
        "do_store": true,
        "weight": 0.3,
        "k": 1.2,
        "b": 0.3
      },
      {
        "name": "body",
        "do_store": true,
        "weight": 0.5,
        "k": 1.2,
        "b": 0.75
      },
      {
        "name": "headingLink",
        "do_store": true,
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
    "lang": "ascii",
    "options": null
  },
  "indexing_config": {
    "num_docs_per_block": 1000,
    "pl_limit": 0,
    "pl_cache_threshold": 0,
    "exclude": [
      "morsels_config.json"
    ],
    "loader_configs": {
      "HtmlLoader": {
        "exclude_selectors": [
          "script,style"
        ],
        "selectors": [
          {
            "attr_map": {},
            "field_name": "title",
            "selector": "title"
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
            "selector": "h1,h2,h3,h4,h5,h6"
          }
        ],
        "type": "HtmlLoader"
      }
    },
    "num_pls_per_dir": 1000,
    "with_positions": false
  }
}
```


