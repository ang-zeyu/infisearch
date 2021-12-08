/* eslint-env node */
const path = require('path');
/* eslint-disable import/no-extraneous-dependencies */
const { merge } = require('webpack-merge');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const TerserPlugin = require('terser-webpack-plugin');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const common = require('./webpack.common');

module.exports = merge(common, {
  mode: 'production',
  output: {
    filename: '[name].bundle.js',
    path: path.resolve(__dirname, 'packages/search-ui/dist'),
    clean: true,
  },
  module: {
    rules: [
      {
        test: /\.css$/i,
        use: [
          MiniCssExtractPlugin.loader,
          'css-loader',
        ],
      },
    ],
  },
  optimization: {
    minimizer: [
      new TerserPlugin(),
    ],
  },
  plugins: [
    new MiniCssExtractPlugin(),
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
      extraArgs: '-- --no-default-features --features lang_latin',
      forceMode: 'production',
      outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_latin'),
    }),
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, './packages/morsels_search'),
      extraArgs: '-- --no-default-features --features lang_chinese',
      forceMode: 'production',
      outDir: path.resolve(__dirname, './packages/morsels_search/pkg/lang_chinese'),
    }),
  ],
});
