import './publicPath';
import { setupWasm, processQuery, getQueryNextN, freeQuery } from './workerSearcher';

// eslint-disable-next-line @typescript-eslint/naming-convention
declare const __indexUrl: string;

async function setupMetadata(): Promise<ArrayBuffer> {
  let cache: Cache;
  try {
    cache = await caches.open(`morsels:${__indexUrl}`);
  } catch {
    // Cache API blocked / unsupported (e.g. firefox private)
  }

  const metadataUrl = `${__indexUrl}metadata.json`;

  return (
    cache
      ? cache.match(metadataUrl)
        .then((resp) => !resp && cache.add(metadataUrl))
        .then(() => cache.match(metadataUrl))
      : fetch(metadataUrl)
  ).then((resp) => resp.arrayBuffer());
}

export default function setupWithWasmModule(wasmModule: Promise<any>) {
  const metadata = setupMetadata();
  
  onmessage = async function worker(ev) {
    const data = ev.data;
    if (data.searcherOptions) {
      await setupWasm(data, metadata, wasmModule);
      postMessage({ isSetupDone: true });
    } else if (data.query) {
      const {
        query, queryId, n, isFree, isGetNextN,
      } = data;
      if (isFree) {
        freeQuery(query, queryId);
      } else if (isGetNextN) {
        const nextResults = getQueryNextN(query, queryId, n);
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
