/* eslint-disable import/no-extraneous-dependencies */
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const path = require('path');

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
    target: 'webworker',
    mode: 'development',
    entry: {
      'search-worker-ascii': getWorkerLangConfig('ascii'),
      'search-worker-latin': getWorkerLangConfig('latin'),
      'search-worker-chinese': getWorkerLangConfig('chinese'),
    },
    output: {
      publicPath: '/',
      filename: '[name].bundle.js',
      path: path.resolve(__dirname, 'packages/search/worker-dist'),
      clean: true,
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
      ],
    },
    plugins: [
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
    ],
  };
};
  
