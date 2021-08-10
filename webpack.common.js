// eslint-disable-next-line import/no-extraneous-dependencies
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const path = require('path');

module.exports = {
  entry: {
    'search-ui': {
      import: './packages/search-ui/src/search.ts',
      filename: 'search-ui.bundle.js',
      library: {
        name: 'initMorsels',
        type: 'umd',
        export: 'default',
      },
    },
  },
  experiments: {
    asyncWebAssembly: true,
  },
  resolve: {
    extensions: ['.ts', '.tsx', '.js'],
  },
  module: {
    rules: [
      {
        test: /\.html$/,
        use: ['html-loader'],
      },
      {
        test: /\.tsx?$/,
        use: ['ts-loader'],
      },
    ],
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
      extraArgs: '-- --no-default-features --features lang_latin',
      forceMode: 'production',
      outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_latin'),
    }),
  ],
};
