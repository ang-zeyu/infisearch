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
    client: {
      webSocketURL: {
        // Reload for the worker won't work when accessing devServer externally
        hostname: 'localhost',
      },
    },
    headers: (() => {
      return env.e2e
        ? { 'Cache-Control': 'no-store' }
        : {};
    })(),
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
      {
        directory: path.join(__dirname, 'packages/search-ui/public/styles'),
        publicPath: '/',
      },
    ],
  },
  optimization: {
    runtimeChunk: {
      name: (entrypoint) => entrypoint.name.includes('worker') ? false : 'runtime',
    },
  },
  output: {
    filename: '[name].bundle.js',
    path: path.resolve(__dirname, 'dist'),
  },
  plugins: (() => {
    const baseHtmlConfig = {
      title: 'Morsels Dev Site',
      scriptLoading: 'blocking',
      favicon: path.join(__dirname, 'packages/search-ui/public/favicon.ico'),
      template: './packages/search-ui/public/template.html',
    };

    return [
      new HtmlWebpackPlugin({
        ...baseHtmlConfig,
        filename: 'index.html',
        chunks: ['search-ui', 'search-ui-light'],
      }),
      new HtmlWebpackPlugin({
        ...baseHtmlConfig,
        filename: 'dark.html',
        chunks: ['search-ui', 'search-ui-dark'],
      }),
    ];
  })(),
});
