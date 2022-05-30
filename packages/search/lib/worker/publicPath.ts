// Regexes from /* webpack/runtime/publicPath */
// Manually handled to accomodate blob url for cross-origin (cdn) worker

// @ts-ignore
__webpack_public_path__ = __morsWrkrUrl.replace(/#.*$/, '')
  .replace(/\?.*$/, '')
  .replace(/\/[^\/]+$/, '/');
