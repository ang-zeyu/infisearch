/* eslint-disable import/no-extraneous-dependencies */
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const { DefinePlugin } = require('webpack');
const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');

const { version } = require('./packages/search/package.json');

function getWorkerLangConfig(lang) {
  return {
    import: path.resolve(__dirname, `packages/search/lib/worker/languages/worker-${lang}.ts`),
    filename: `search-worker-${lang}.bundle.js`,
  };
}

module.exports = (env) => {
  const perfOption = env.perf ? ',perf' : '';
  const perfMode = env.perf ? { forceMode: 'production' } : {};

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
      'search-worker-ascii': getWorkerLangConfig('ascii'),
      'search-worker-latin': getWorkerLangConfig('latin'),
      'search-worker-chinese': getWorkerLangConfig('chinese'),
      'search-ui-light': {
        import: path.resolve(__dirname, 'packages/search-ui/src/styles/light.css'),
      },
      'search-ui-dark': {
        import: path.resolve(__dirname, 'packages/search-ui/src/styles/dark.css'),
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
        {
          test: /\.css$/i,
          use: [
            MiniCssExtractPlugin.loader,
            'css-loader',
          ],
        },
      ],
    },
    plugins: [
      new MiniCssExtractPlugin(),
      new WasmPackPlugin({
        crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
        extraArgs: '-- --no-default-features --features lang_ascii' + perfOption,
        outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_ascii'),
        ...perfMode,
      }),
      new WasmPackPlugin({
        crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
        extraArgs: '-- --no-default-features --features lang_latin' + perfOption,
        outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_latin'),
        ...perfMode,
      }),
      new WasmPackPlugin({
        crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
        extraArgs: '-- --no-default-features --features lang_chinese' + perfOption,
        outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_chinese'),
        ...perfMode,
      }),
      new DefinePlugin({
        MORSELS_VERSION: `'${version}'`,
      }),
    ],
  };
};
