# What's Next

This section briefly details some future plans for this project, organised roughly in decreasing priority:

## Cutting Down the Wasm Binary Size

Not much to be said here - the smallest wasm binary is `992KB`, and `334KB` when gzipped.

Definitely something to work on, although the silver lining (hopefully) is that users typically don't need to immediately (within < 1s of page load?) access search functionalities.

## Search API

I.e. publishing `@morsels/search-lib`. Also, stabilizing some finer related details of the [renderers API](./search_configuration_renderers.md) available.

## Field Sorting & Types

While the focus currently is on free text queries sorted by relevance (BM25), it would definitely be great (if feasible) to support use cases that need to sort by some other fields.

Field types (e.g. integers), and sorting based on these types may be supported as such in the future.

The primary challenge would be doing this in a slightly scalable manner (i.e. retrieving "morsels" of data), instead of requiring that all field data be present immediately.

## Proximity Queries

Basically something like [this](https://www.guidingtech.com/16116/google-search-little-known-around-operator/):

```
lorem AROUND(2) dolor
```

## Dynamic Linking for Language Modules

Language modules are "bundled" into the `morsels_search` wasm module at the moment. In order to reduce binary size, each module is configured via feature flags and bundled separately. This however does mean that any and all language modules have to be PR-ed to the upstream repo (otherwise, one would have to maintain a separate fork of morsels on its own distribution channels). This is pending better dynamic linking support in wasm.

## Exposing `morsels_indexer` Package API

The plan is for the morsels_indexer package to also expose its API. This is still a WIP at this point, and I'm not exactly sure if there would even be a demand for this given the very specific use cases of this tool -- raise an issue if so!
