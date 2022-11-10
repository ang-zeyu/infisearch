import '../publicPath';
import setupWithWasmModule from '../worker';

// eslint-disable-next-line import/no-extraneous-dependencies
setupWithWasmModule(import(
  /* webpackMode: "eager" */
  /* webpackExports: ["get_new_searcher", "get_query"] */
  '@infisearch/lang-chinese'
));
