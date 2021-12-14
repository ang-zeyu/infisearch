# Developing

## Building and Running the Development Site

Once you have you test files placed in the correct folder per the previous chapter, run the `npm run index1` script to index your content, then run `npm run devServer1` to serve up the indexer's output on port `3000`.

Finally, use the `npm run dev` script to open the development site on port `8080`. This script runs the webpack build process, which in turn triggers the WebAssembly build process as well through the wasm-pack webpack plugin.

There are also `npm run index2` / `devServer2` scripts available. An interactive script may be added in the future, but for now 2 should suffice (1 to test a smaller site, 1 to test a very large collection).

## Working With the mdbook-morsels Plugin / Documentation Edits

Simply run `npm run devDocsFull` to get serve up the documentation on port `8000`!

This script runs the following subscripts:
1. `npm run installIndexer`, this builds and install the indexer command-line tool to your `PATH` (mdbook plugins are basically separate executables, operating on `stdin / stdout`).

1. `npm run buildSearch`, which builds the search-ui bundles, to ensure the documentation is being developed on the latest changes.

1. `npm run installMdbook`, which builds and installs the mdbook plugin executable.

1. `npm run devDocs`, which serves up the documentation at port `8000`. The commands are segregated to reduce iterative build times if you only need to execute the last few script(s).

You can also run the individual commands separately to reduce iterative build times if you only need to execute the last few script(s).
