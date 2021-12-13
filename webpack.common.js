/* eslint-disable import/no-extraneous-dependencies */
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const { DefinePlugin } = require('webpack');
const path = require('path');

const { version } = require('./packages/search/package.json');

module.exports = (env) => {
  return {
    entry: {
      'search-ui': {
        import: path.resolve(__dirname, 'packages/search-ui/src/search.ts'),
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
          test: /\.tsx?$/,
          use: ['ts-loader'],
        },
      ],
    },
    plugins: [
      !env.perf
        ? new WasmPackPlugin({
          crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
          extraArgs: '-- --no-default-features --features lang_ascii',
          outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_ascii'),
        })
        : new WasmPackPlugin({
          crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
          extraArgs: '-- --no-default-features --features lang_ascii,perf',
          forceMode: 'production',
          outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_ascii'),
        }),
      new DefinePlugin({
        MORSELS_VERSION: `'${version}'`,
      }),
    ],
  };
};
