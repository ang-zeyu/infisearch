import setupWithWasmModule from '../worker';

// eslint-disable-next-line import/no-extraneous-dependencies
setupWithWasmModule(import(
  /* webpackMode: "eager" */
  './wasm-ascii'
));
