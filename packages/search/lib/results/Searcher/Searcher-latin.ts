// @ts-ignore
import workerScript from '../../../worker-dist/search-worker-latin.bundle?raw';
import Searcher from '../../results/Searcher';
import { workerScript as SearcherScript } from '../../results/Searcher';
import Query from '../../results/Query';

SearcherScript.s = workerScript;

export {
  Searcher,
  Query,
};
