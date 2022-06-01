import './publicPath';
import WorkerSearcher from './workerSearcher';

export default function setupWithWasmModule(wasmModule: Promise<any>) {
  let workerSearcher: WorkerSearcher;
  
  onmessage = async function worker(ev) {
    if (ev.data.searcherOptions) {
      // const now = performance.now();
  
      workerSearcher = await WorkerSearcher._mrlSetup(ev.data, wasmModule);
      postMessage({ isSetupDone: true });
  
      // console.log(`Worker setup took ${performance.now() - now} ms`);
    } else if (ev.data.query) {
      const {
        query, queryId, n, isFree, isGetNextN,
      } = ev.data;
      if (isFree) {
        workerSearcher._mrlFreeQuery(query, queryId);
      } else if (isGetNextN) {
        const nextResults = workerSearcher._mrlGetQueryNextN(query, queryId, n);
        postMessage({
          query,
          queryId,
          nextResults,
        });
      } else {
        const workerQuery = await workerSearcher._mrlProcessQuery(query, queryId);
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
