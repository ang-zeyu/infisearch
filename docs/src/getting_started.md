# Getting Started

This page assumes the use case of a **static site**, that is:
- You have some `.html` files you want to index.
- These `.html` files are served in a static file server, and are accessible by `@morsels/search-ui` to generate result previews.
- You have an `<input>` element to attach a search dropdown to.
  - For mobile devices, a fullscreen UI will show instead when this input element is focused.
  - Note that this documentation is using an alternative UI mode (try out the search function!), which is covered later under [search configuration](./search_configuration.md#ui-mode).
    To preview the default mode, head on over to the mdbook page [here](./getting_started_mdbook.md#preview), and click on the **auto** button.

If you require more, have a look through here first, then head on over to the subsequent configuration pages.

## Installing the indexer

There are two options here:
- If you have the rust / cargo toolchains setup, simply run `cargo install morsels_indexer`!
- Alternatively, the cli binaries are also available [here](https://github.com/ang-zeyu/morsels/releases).

## Running the indexer

Run the executable as such, replacing `<source-folder-path>` with the relative or absolute folder path of your source html files, and `<output-folder-path>` with your desired index output folder.

```
morsels <source-folder-path> <output-folder-path>
```

If you are using the binaries, replace `morsels` with the appropriate executable name.

### Other Cli Options

- `--config-init`: While optional, if it is your first time running the tool, you can run the above command with this flag, then **run it again without this flag**.
This flag outputs the default `morsels_config.json` that can be used to [configure the indexer](./indexer_configuration.md) later on, and does not perform any indexing.
- `-c <config-file-path>`: You may also change the config file location (relative to the `source-folder-path`) using the `-c <config-file-path>` option.
- `--preserve-output-folder`: **All existing contents** in the output folder are also **removed** when running a full index. Specify this option to avoid this.

## Installing the search library / UI

### Installation via CDN

```html
<!-- Replace "v0.0.2" as appropriate -->

<!--  Search UI package script, which bundles the search library together with it -->
<script src="https://cdn.jsdelivr.net/gh/ang-zeyu/morsels@v0.0.2/packages/search-ui/dist/search-ui.bundle.js"></script>
<!-- Search UI css, this provides very basic styling for the search dropdown, and can be omitted if desired -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/ang-zeyu/morsels@v0.0.2/packages/search-ui/dist/search-ui.css" />
```

> ⚠️ Ensure the versions here **tally with the indexer version** used.

#### Hosting the Files Locally

If you wish to serve the files locally instead, you can find the necessary files in the release packages [here](https://github.com/ang-zeyu/morsels/releases). All files inside `search.morsels.zip` are required, and their functions are as follows:

- `search-ui.bundle.js`
- `search-ui.css`
- `search.worker.bundle.js`
- Multiple (as many supported languages / tokenizers as there are):
  - `wasm.lang-latin-index-js.bundle`
  - an accompanying wasm binary

`search.worker.bundle.js` and the `.wasm` files are expected to be **accessible in the same folder** relative to the linked `search-ui.bundle.js`.

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

This **requires an input element** with an id of `morsels-search` to be present in the page by default, which can be configured via `uiOptions.input`.

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

## What's Next

That's it! Head on over to the search configuration chapter to learn more about configuring the UI behaviours / outputs.
The indexer configuration chapters covers a wide range of topics such as adding additional fields to index, mapping file contents to fields, and language configurations.
