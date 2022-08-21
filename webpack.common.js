/* eslint-disable import/no-extraneous-dependencies */
const { DefinePlugin } = require('webpack');
const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');

const { version } = require('./packages/search/package.json');

function getLangConfig(lang) {
  return {
    import: path.resolve(__dirname, `packages/search-ui/src/entries/${lang}.ts`),
    filename: `search-ui.${lang}.bundle.js`,
    library: {
      name: 'initMorsels',
      type: 'umd',
      export: 'default',
    },
  };
}

// All entry points except the webworker/wasm
module.exports = {
  entry: {
    'search-ui-ascii': getLangConfig('ascii'),
    'search-ui-latin': getLangConfig('latin'),
    'search-ui-chinese': getLangConfig('chinese'),
    'search-ui-basic': {
      import: path.resolve(__dirname, 'packages/search-ui/src/styles/basic.css'),
    },
    'search-ui-light': {
      import: path.resolve(__dirname, 'packages/search-ui/src/styles/light.css'),
    },
    'search-ui-dark': {
      import: path.resolve(__dirname, 'packages/search-ui/src/styles/dark.css'),
    },
  },
  output: {
    publicPath: '/',
  },
  resolve: {
    extensions: ['.ts', '.tsx', '.js'],
  },
  module: {
    rules: [
      {
        oneOf: [
          {
            resourceQuery: /raw/,
            type: 'asset/source',
          },
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
    ],
  },
  plugins: [
    new MiniCssExtractPlugin(),
    new DefinePlugin({
      MORSELS_VERSION: `'${version}'`,
    }),
  ],
};
