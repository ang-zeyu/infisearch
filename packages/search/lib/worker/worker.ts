import './publicPath';
import { setupWasm, processQuery, getQueryNextN, freeQuery } from './workerSearcher';


export default function setupWithWasmModule(wasmModule: Promise<any>) {
  onmessage = async function worker(ev) {
    const data = ev.data;
    if (data.searcherOptions) {
      await setupWasm(data, wasmModule);
      postMessage({ isSetupDone: true });
    } else if (data.query) {
      const {
        query, queryId, n, isFree, isGetNextN,
      } = data;
      if (isFree) {
        freeQuery(queryId);
      } else if (isGetNextN) {
        const nextResults = getQueryNextN(queryId, n);
        postMessage({
          query,
          queryId,
          nextResults,
        });
      } else {
        const workerQuery = await processQuery(query, queryId);
        postMessage({
          query,
          queryId,
          searchedTerms: workerQuery._mrlSearchedTerms,
          queryParts: workerQuery._mrlQueryParts,
        });
      }
    }
  };

  // Initialised onmessage handler, ask for searcherOptions
  postMessage('');
}
