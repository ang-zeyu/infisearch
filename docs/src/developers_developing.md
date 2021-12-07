# Developing

## Building and Running the Development Site

Once you have you test files placed in the correct folder per the previous chapter, run the `npm run index1` script to index your content, then run `npm run devServer1` to serve up the indexer's output on port `3000`.

Finally, use the `npm run dev` script to open the development site on port `8080`. This script runs the webpack build process, which in turn triggers the WebAssembly build process as well through the wasm-pack webpack plugin.

## Working With the mdbook-morsels Plugin

As mdbook plugins are basically separate executables, operating on `stdin / stdout`, you'll need to first run `npm run installIndexer` to build and install the indexer command-line tool to your `PATH`. (including whenever changes are made to the indexer code)

Next, run the `buildSearch` then `buildMdbook` script, which builds the search binary and mdbook executables respectively. Lastly, execute `devDocs`, which serves up the documentation at port `8000`. The commands are segregated to reduce iterative build times if you only need to execute the last few script(s).
