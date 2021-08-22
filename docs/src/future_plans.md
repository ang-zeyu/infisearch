# Future Plans

This section briefly details some future plans for this project:

## Exposing `morsels_indexer` Package API

The plan is for the morsels_indexer package to also formalise its API. This is still a WIP at this point, and I'm not exactly sure if there would even be a demand for this given the very specific nature of this tool -- raise an issue if so!

## Dynamic Linking for Language Modules

Language modules are "bundled" into the `morsels_search` wasm module at the moment. In order to reduce binary size, each module is configured via feature flags and bundled separately. This however does mean that any and all language modules have to be PR-ed to the upstream module (otherwise, one would have to maintain a separate fork of morsels on its own distribution channels).

This is mainly pending dynamic linking in wasm ala llvm / rust which is still finicky and unconfirmed at the moment.

## Proximity Queries

Basically something like [this](https://www.guidingtech.com/16116/google-search-little-known-around-operator/):

```
lorem AROUND(2) dolor
```
