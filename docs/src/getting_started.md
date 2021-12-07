# Getting Started

This page assumes the following use case:
- You have some `.html` files you want to index
- These `.html` files are served in a static file server, and are accessible by the search-ui to generate result previews

If you require more, have a look through here first, then jump into the subsequent configuration pages to learn more.

## Installing the indexer

If you have the rust / cargo toolchain setup, simply run `cargo install morsels_indexer`.

The cli binaries are also available [here](https://github.com/ang-zeyu/morsels/releases) if preferred.


## Running the indexer

Run the executable as such, replacing `<source-folder-path>` with the relative or absolute folder path of your source html files, and `<output-folder-path>` with your desired index output folder.

```
morsels <source-folder-path> <output-folder-path>
```

### Other Cli Options

While optional, if it is your first time running the tool, you can run the above command with the `--init` or `-i` flag, then run it again without this flag.
This flag outputs the default `_morsels_config.json` that can be used to [configure the indexer](./indexing_configuration.md) later on.

You may also change the config file location (relative to the `source-folder-path`) using the `-c <config-file-path>` option.


## Installing the search library / UI

### Installation via CDN

```html
<!-- Replace "version" as appropriate -->

<!--  Search UI package script, which bundles the search library together with it -->
<script src="https://cdn.jsdelivr.net/npm/morsels-search-ui@version/search-ui.bundle.js"></script>
<!-- Search UI css, this provides very basic styling for the search dropdown, and can be omitted if desired -->
<script src="https://cdn.jsdelivr.net/npm/morsels-search-ui@version/search-ui.css"></script>
```

#### Hosting the Files Locally

If you wish to serve the files locally instead, you can find the latest versions in [this folder](https://github.com/ang-zeyu/morsels/tree/main/packages/search-ui/dist), or in the release packages [here](https://github.com/ang-zeyu/morsels/releases).

The following files will be present in each release:

- `search-ui.bundle.js`
- `search-ui.css`
- `search.worker.bundle.js`
- Multiple (as many supported languages / tokenizers as there are):
  - `wasm.lang-latin-index-js.bundle`
  - an accompanying wasm binary

Note that `search.worker.bundle.js` and the wasm files are expected to be accessible in the same folder relative to the linked `search-ui.bundle.js`.

### Installation via Bundlers

As morsels consists of a javascript (typescript) and rust portion enabled by WebAssembly, including it into your project's bundling / build process is likely infeasible, as rust / wasm compilation takes a lot of time (and requires [extra toolchains](./developers_setting_up.md)).

Instead, use the file copying functionalities of your bundler to copy morsels' assets into the appropriate output directories.


For example, using the [CopyWebpackPlugin](https://webpack.js.org/plugins/copy-webpack-plugin/), the following (untested) setup should be all you need:

```js
// Under plugins configuration
new CopyPlugin({
  patterns: [
    {
      from: path.join(require.resolve('@morsels/search-ui'), 'dist'),
      to: "dest" // change as appropriate
    },
  ],
})
```


```html
<!-- Replace links as appropriate -->
<script src=".../search-ui.bundle.js"></script>
<script src=".../search-ui.css"></script>
```

### Initialisation Call

Once you have loaded the bundles, to initialise morsels, simply call the `initMorsels` function.

This requires an input element with an id of `morsels-search` to be present in the page by default.

```ts
initMorsels({
  searcherOptions: {
    // Output folder url specified as the second parameter in the cli command
    url: 'http://192.168.10.132:3000/output/',
  },
  uiOptions: {
    // Input / source folder url, specified as the first parameter in the cli command
    sourceFilesUrl: 'http://192.168.10.132:3000/source/',
  }
});
```
