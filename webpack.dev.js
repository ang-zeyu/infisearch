/* eslint-env node */
const path = require('path');
/* eslint-disable import/no-extraneous-dependencies */
const { merge } = require('webpack-merge');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const common = require('./webpack.common');

module.exports = (env) => merge(common(env), {
  mode: 'development',
  devtool: 'inline-source-map',
  devServer: {
    hot: true,
    host: '0.0.0.0',
    open: false,
    static: [
      {
        directory: path.join(__dirname, 'test_files/1'),
        publicPath: '/1',
      },
      {
        directory: path.join(__dirname, 'test_files/2'),
        publicPath: '/2',
      },
      {
        directory: path.join(__dirname, 'test_files/3'),
        publicPath: '/3',
      },
      {
        directory: path.join(__dirname, 'e2e'),
        publicPath: '/e2e',
      },
    ],
  },
  output: {
    filename: '[name].bundle.js',
    path: path.resolve(__dirname, 'dist'),
  },
  module: {
    rules: [
      {
        test: /\.css$/i,
        use: [
          'style-loader',
          'css-loader',
        ],
      },
    ],
  },
  plugins: [
    new HtmlWebpackPlugin({
      scriptLoading: 'blocking',
      template: './packages/search-ui/public/template.html',
    }),
  ],
});
