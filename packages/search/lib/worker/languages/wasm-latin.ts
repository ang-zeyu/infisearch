/*
 Webpack dynamic imports, even if eager, re-resolve public path automatically.

 This file therefore "statically" imports the wasmModule with the correct public path.
 It is then dynamically re-imported in worker-*.ts
*/

import '../publicPath';
// eslint-disable-next-line import/no-extraneous-dependencies
export { get_query, get_new_searcher } from '@morsels/lang-latin';
