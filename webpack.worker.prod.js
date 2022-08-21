/* eslint-disable import/no-extraneous-dependencies */
const { merge } = require('webpack-merge');
const TerserPlugin = require('terser-webpack-plugin');
const common = require('./webpack.worker');

module.exports = (env) => merge(common(env), {
  mode: 'production',
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
    ],
  },
});
