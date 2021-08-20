import WorkerSearcher from './workerSearcher';

let workerSearcher: WorkerSearcher;

onmessage = async function worker(ev) {
  if (ev.data.searcherOptions) {
    // const now = performance.now();

    workerSearcher = await WorkerSearcher.setup(ev.data);
    postMessage({ isSetupDone: true });

    // console.log(`Worker setup took ${performance.now() - now} ms`);
  } else if (ev.data.query) {
    const {
      query, timestamp, n, isFree, isGetNextN,
    } = ev.data;
    if (isFree) {
      workerSearcher.freeQuery(query, timestamp);
    } else if (isGetNextN) {
      const nextResults = workerSearcher.getQueryNextN(query, timestamp, n);
      postMessage({
        query,
        timestamp,
        nextResults,
      });
    } else {
      const workerQuery = await workerSearcher.processQuery(query, timestamp);
      postMessage({
        query,
        timestamp,
        searchedTerms: workerQuery.searchedTerms,
        queryParts: workerQuery.queryParts,
      });
    }
  }
};
