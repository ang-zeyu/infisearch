import WorkerSearcher from './workerSearcher';

let workerSearcher: WorkerSearcher;

onmessage = async function worker(ev) {
  if (ev.data.searcherOptions) {
    const now = performance.now();

    workerSearcher = await WorkerSearcher.setup(ev.data);
    postMessage({ isSetupDone: true });

    console.log(`Worker setup took ${performance.now() - now} ms`);
  } else if (ev.data.query) {
    const {
      query, n, isFree, isGetNextN,
    } = ev.data;
    if (isFree) {
      workerSearcher.freeQuery(query);
    } else if (isGetNextN) {
      const nextResults = workerSearcher.getQueryNextN(query, n);
      postMessage({
        query,
        nextResults,
      });
    } else {
      const workerQuery = await workerSearcher.processQuery(query);
      postMessage({
        query,
        searchedTerms: workerQuery.searchedTerms,
        queryParts: workerQuery.queryParts,
      });
    }
  }
};
