// eslint-disable-next-line import/no-extraneous-dependencies
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const path = require('path');

module.exports = {
  entry: {
    main: './packages/search/lib/search.ts',
    worker: './packages/search/lib/worker/worker.ts',
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
        test: /\.html$/,
        use: ['html-loader'],
      },
      {
        test: /\.css$/,
        use: [
          'style-loader',
          'css-loader',
        ],
      },
      {
        test: /\.tsx?$/,
        use: ['ts-loader'],
      },
      {
        test: /\.(svg|png|jpg|gif)$/,
        use: {
          loader: 'file-loader',
          options: {
            name: '[name].[hash].[ext]',
            outputPath: 'imgs',
          },
        },
      },
    ],
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, './packages/librarian_search'),
      extraArgs: '-- --no-default-features --features lang_latin',
      forceMode: 'production',
      outDir: path.resolve(__dirname, './packages/librarian_search/pkg/lang_latin'),
    }),
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, './packages/librarian_search'),
      extraArgs: '-- --no-default-features --features lang_chinese',
      forceMode: 'production',
      outDir: path.resolve(__dirname, './packages/librarian_search/pkg/lang_chinese'),
    }),
  ],
};
