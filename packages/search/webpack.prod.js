/* eslint-env node */
const path = require('path');
/* eslint-disable import/no-extraneous-dependencies */
const merge = require('webpack-merge');
const TerserPlugin = require('terser-webpack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const common = require('./webpack.common');

module.exports = merge(common, {
  mode: 'production',
  output: {
    filename: '[name].[contentHash].bundle.js',
    path: path.resolve(__dirname, 'dist'),
  },
  optimization: {
    minimizer: [
      new TerserPlugin(),
      new HtmlWebpackPlugin({
        template: './public/template.html',
        minify: {
          removeAttributeQuotes: true,
          collapseWhitespace: true,
          removeComments: true,
        },
      }),
      new WasmPackPlugin({
        crateDirectory: path.resolve(__dirname, '../librarian_search'),
      }),
    ],
  },
});
