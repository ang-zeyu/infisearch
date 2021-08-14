# Getting Started

Getting started with the default setup is fairly easy.

The default setup assumes you:
- Have some `.html` files you want to index
- These `.html` files are served in a static file server, and are accessible by the search-ui to generate result previews

If you need more than this, have a look through here first, then jump into the [indexer configuration](indexing_configuration.md) page to learn more.

## Installing the indexer

If you have the rust / cargo toolchain setup, simply run `cargo install morsels_indexer`.

If not, the cli binaries are also available [here](https://github.com/ang-zeyu/morsels/releases).

## Running the indexer

```
morsels_indexer <source-folder-path> <output-folder-path>
```

If it is your first time running the tool, first run the below command with the `--init` or `-i` flag, then run the command again without the flag.
This option outputs a default [`_morsels_config.json`](./chapter_5.md) that can be used to configure the indexer later on.

You may also change the config file location (relative to the `source-folder-path`) using the `-c <config-file-path>` option.


## Installing the search library / UI

Installation for this section may very greatly depending on your setup.

For now, let's assume you just need the default settings.

### Installation via cdn

```html
<!-- Replace "version" as appropriate -->

<!--  Search UI package script, which bundles the search library together with it -->
<script src="https://cdn.jsdelivr.net/npm/morsels-search-ui@version/search-ui.bundle.js"></script>
<!-- Search UI css, this provides very basic styling for the search dropdown, and can be omitted if desired -->
<script src="https://cdn.jsdelivr.net/npm/morsels-search-ui@version/search-ui.css"></script>
```

To initialise morsels, call the `initMorsels` function as exposed by `search-ui.bundle.js`:

```js
initMorsels({
    searcherOptions: {
        // Output folder url specified as the second parameter in the cli command
        url: 'http://192.168.10.132:3000/output',
    },
    // Input / source folder url, specified as the first parameter in the cli command
    sourceFilesUrl: 'http://192.168.10.132:3000/source'
});
```

### Installation via webpack

<p style="color: red;">This section is not yet completed!</p>

With some setup, the library may also be integrated via bundlers. This guide will only cover webpack.

The following section shows how to include the worker bundle and `.wasm` files using the [CopyWebpackPlugin](https://webpack.js.org/plugins/copy-webpack-plugin/).

The main `search-ui.bundle.js` will be bundled into your application.

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

#### Bundling Everything

Alternatively, if you wish to bundle the `.wasm` binaries from source, you'll need to install [wasm pack plugin](https://github.com/wasm-tool/wasm-pack-plugin) and its various dependencies, and adapt the configurations used in `webpack.common.js` in the github repo. This is unrecommended however, as it adds global dependency requirements (i.e. rust compiler) to your development toolchain and may slow build times significantly.
