# Getting Started

Getting started with the following default setup is fairly easy straightforward. It assumes:
- You have some `.html` files you want to index
- These `.html` files are served in a static file server, and are accessible by the search-ui to generate result previews

If you need more than this, have a look through here first, then jump into the [configuration](search_configuration.md) page to learn more.

## Installing the indexer

If you have the rust / cargo toolchain setup, simply run `cargo install morsels_indexer`.

If not, the cli binaries are also available [here](https://github.com/ang-zeyu/morsels/releases).


## Running the indexer

```
morsels <source-folder-path> <output-folder-path>
```

While optional, if it is your first time running the tool, you can run the above command with the `--init` or `-i` flag, then run it again without this flag.
This flag outputs the default [`_morsels_config.json`](./indexing_configuration.md) that can be used to configure the indexer later on.

You may also change the config file location (relative to the `source-folder-path`) using the `-c <config-file-path>` option.


## Installing the search library / UI

### Installation via cdn 

```html
<!-- Replace "version" as appropriate -->

<!--  Search UI package script, which bundles the search library together with it -->
<script src="https://cdn.jsdelivr.net/npm/morsels-search-ui@version/search-ui.bundle.js"></script>
<!-- Search UI css, this provides very basic styling for the search dropdown, and can be omitted if desired -->
<script src="https://cdn.jsdelivr.net/npm/morsels-search-ui@version/search-ui.css"></script>
```


If you wish to serve the files locally instead, you can find the latest versions in [this folder](https://github.com/ang-zeyu/morsels/tree/main/packages/search-ui/dist), or in the release packages [here](https://github.com/ang-zeyu/morsels/releases).

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

Once you have loaded the bundles, to initialise morsels, simply call the `initMorsels` function as exposed globally by `search-ui.bundle.js`.

This requires an input element with an id of `morsels-search` to be present in the page by default.

```js
initMorsels({
    searcherOptions: {
        // Output folder url specified as the second parameter in the cli command
        url: 'http://192.168.10.132:3000/output/',
    },
    // Input / source folder url, specified as the first parameter in the cli command
    sourceFilesUrl: 'http://192.168.10.132:3000/source/'
});
```
