/* eslint-env node */
const path = require('path');
/* eslint-disable import/no-extraneous-dependencies */
const { merge } = require('webpack-merge');
const CssMinimizerPlugin = require('css-minimizer-webpack-plugin');
const RemovePlugin = require('remove-files-webpack-plugin');
const TerserPlugin = require('terser-webpack-plugin');
const common = require('./webpack.common');

module.exports = (env) => merge(common(env), {
  mode: 'production',
  output: {
    filename: '[name].bundle.js',
    path: path.resolve(__dirname, 'packages/search-ui/dist'),
    clean: true,
  },
  optimization: {
    minimizer: [
      new TerserPlugin({
        terserOptions: {
          compress: {},
          mangle: {
            properties: {
              regex: /^_mrl\w+/,
            },
          },
        },
      }),
      new CssMinimizerPlugin(),
    ],
  },
  plugins: [
    // https://github.com/webpack-contrib/mini-css-extract-plugin/issues/151
    new RemovePlugin({
      after: {
        root: path.resolve(__dirname, 'packages/search-ui/dist'),
        include: [
          'search-ui-basic.bundle.js',
          'search-ui-light.bundle.js',
          'search-ui-dark.bundle.js',
        ],
      },
    }),
  ],
});
