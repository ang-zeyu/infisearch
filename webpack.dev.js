/* eslint-env node */
const path = require('path');
/* eslint-disable import/no-extraneous-dependencies */
const { merge } = require('webpack-merge');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const common = require('./webpack.common');

module.exports = (env) => merge(common, {
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
        directory: path.join(__dirname, 'packages/search/worker-dist'),
        publicPath: '/',
      },
      {
        directory: path.join(__dirname, 'test_files/1'),
        publicPath: '/1',
        watch: false,
      },
      {
        directory: path.join(__dirname, 'test_files/2'),
        publicPath: '/2',
        watch: false,
      },
      {
        directory: path.join(__dirname, 'test_files/3'),
        publicPath: '/3',
        watch: false,
      },
      {
        directory: path.join(__dirname, 'e2e'),
        publicPath: '/e2e',
        watch: false,
      },
      {
        directory: path.join(__dirname, 'packages/search-ui/public/static'),
        publicPath: '/',
      },
      {
        directory: path.join(__dirname, 'docs/book/html'),
        publicPath: '/docs',
        watch: false,
      },
    ],
  },
  output: {
    filename: '[name].bundle.js',
    path: path.resolve(__dirname, 'dist'),
  },
  plugins: (() => {
    const baseHtmlConfig = {
      title: 'InfiSearch Dev Site',
      scriptLoading: 'blocking',
      favicon: path.join(__dirname, 'packages/search-ui/public/favicon.ico'),
      template: './packages/search-ui/public/template.html',
    };

    const themes = ['basic', 'light', 'dark'];
    const languages = ['ascii', 'latin', 'chinese'];

    const plugins = [];
    for (const theme of themes) {
      for (const language of languages) {
        plugins.push(new HtmlWebpackPlugin({
          ...baseHtmlConfig,
          filename: `${theme}-theme_${language}-lang.html`,
          chunks: [`search-ui-${language}`, `search-ui-${theme}`],
        }));
      }
    }

    return plugins;
  })(),
});
