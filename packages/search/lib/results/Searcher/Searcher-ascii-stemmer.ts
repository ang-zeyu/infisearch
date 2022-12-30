// @ts-ignore
import workerScript from '../../../worker-dist/search-worker-ascii-stemmer.bundle?raw';
import Searcher from '../Searcher';
import { workerScript as SearcherScript } from '../Searcher';
import Query from '../Query';

SearcherScript.s = workerScript;

export {
  Searcher,
  Query,
};
