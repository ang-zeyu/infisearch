# Introduction

Morsels is a client-side search library / cli build tool meant for use in modern (ES6+) browsers.

The difference with other search libraries like lunr.js and elasticlunr.js is that this tool is focused on providing **scalability** (but not infinitely so) using a pre-built index split into many small files. Libraries like lunr.js also have the option of prebuilt indexes, but far too often, they come in a monolithic format. This approach is simply not practical when indexing pure text collections over `> 100MB`.

This does mean that it is **not possible** to use morsels for client-side indexing + searching. If this is the use case, do consider other existing and mature libraries like lunr.js.

In short, this tool is tailored for a very specific audience:
- You have a fairly large collection of html, csv, or json (for now, only these are supported) files (`> 100MB`)
- You don't want or can't run a search server / search Saas (eg. Algolia docsearch)
- You don't want the user downloading and loading a gigantic `> 50mb` index, blowing up memory usage everytime they visit your site.
- ES5 support is not a concern (simply not possible with the technologies used here)

## Limits

The test collection used during development is a pure-text `380mb` .csv file, with positional indexing enabled.

As an **estimate**, this library should be able to handle collections < `800mb` with positional indexing. Without it, you could potentially index collections > `2gb` in size (the index size shrinks 3 to 4 fold without this).

## Libraries

This project is made up of 3 crates and 2 packages.

- morsels_indexer: the cli tool providing indexing functionalities for several file formats
- morsels_search: the rust wasm crate providing search functionalities
- morsels_common: library containing internal common functionalities for the above 2 crates..

- @morsels/search-lib: a small companion library to morsels_search for interfacing with the wasm crate. This may be used without the `morsels-searchui` package below in the future.
- @morsels/search-ui: interfaces with @morsels/search-lib to provide basic search UI functionalities (e.g. SERP generation)
