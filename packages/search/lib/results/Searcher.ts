import Query from './Query';
import { SearcherOptions, MorselsConfig } from './Config';
import Result from './Result';
import { QueryPart } from '../parser/queryParser';
import PersistentCache from './Cache';
import { getFieldUrl } from '../utils/FieldStore';

declare const MORSELS_VERSION;

class Searcher {
  config: MorselsConfig;

  isSetupDone: boolean = false;

  readonly setupPromise: Promise<any>;

  private worker: Worker;

  private queries: {
    [query: string]: {
      [queryId: number]: {
        promise: Promise<any>,
        resolve: any,
      }
    }
  } = Object.create(null);

  private nextId = 0;

  private cache: PersistentCache;

  constructor(private options: SearcherOptions) {
    this.worker = new Worker(new URL(
      /* webpackChunkName: "search.worker" */
      '../worker/worker.ts', import.meta.url,
    ));

    const workerSetup: Promise<void> = new Promise((resolve) => {
      this.worker.onmessage = (ev) => {
        if (ev.data.isSetupDone) {
          resolve();
        } else if (ev.data.query) {
          const {
            query,
            queryId,
            nextResults,
            searchedTerms,
            queryParts,
          } = ev.data;

          this.queries[query][queryId].resolve({
            query,
            nextResults,
            searchedTerms,
            queryParts,
          });
        }
      };

      this.worker.onmessageerror = (ev) => {
        console.log(ev);
      };
    });

    const cacheName = `morsels:${options.url}`;

    this.setupPromise = this.retrieveConfig()
      .then(() => this.setupCache(cacheName))
      .then(() => this.worker.postMessage(this.config))
      .then(() => workerSetup)
      .then(() => {
        this.setupFieldStoreCache();
        this.setupIndexCache();
      })
      .then(() => this.isSetupDone = true);
  }

  private async setupCache(cacheName: string) {
    try {
      let cache = await caches.open(cacheName);
      const cacheIndexVerResp = await cache.match('/index_ver');
      if (cacheIndexVerResp) {
        const cacheIndexVer = await cacheIndexVerResp.text();
        if (this.config.indexVer !== cacheIndexVer) {
          await caches.delete(cacheName);
          cache = await caches.open(cacheName);
        }
      }

      await cache.put('/index_ver', new Response(this.config.indexVer));
      this.cache = new PersistentCache(cache);
    } catch {
      // Cache API blocked / unsupported (e.g. firefox private)
      this.cache = new PersistentCache(undefined);
    }
  }

  private setupFieldStoreCache() {
    if (!this.options.cacheAllFieldStores) {
      return;
    }

    // These 2 parameters are "clean" multiples / divisors of each other
    const { fieldStoreBlockSize, indexingConfig } = this.config;
    const increment = Math.min(fieldStoreBlockSize, indexingConfig.numDocsPerBlock);
    for (let docId = 0; docId < this.config.lastDocId; docId += increment) {
      this.cache.cacheJson(getFieldUrl(this.options.url, docId, this.config));
    }
  }

  private setupIndexCache() {
    const pls = this.config.indexingConfig.plNamesToCache;
    pls.forEach((pl) => {
      const folder = Math.floor(pl / this.config.indexingConfig.numPlsPerDir);
      const url = `${this.options.url}pl_${folder}/pl_${pl}.json`;
      this.cache.cacheUrl(url);
    });
  }

  private async retrieveConfig(): Promise<void> {
    this.config = await (await fetch(`${this.options.url}morsels_config.json`)).json();

    if (this.config.ver !== MORSELS_VERSION) {
      throw new Error('Morsels search !== indexer version!');
    }

    if (!('cacheAllFieldStores' in this.options)) {
      this.options.cacheAllFieldStores = !!this.config.cacheAllFieldStores;
    }

    this.options.useQueryTermProximity = this.options.useQueryTermProximity
        && this.config.indexingConfig.withPositions;

    this.config.searcherOptions = this.options;
  }

  private deleteQuery(query: string, queryId: number) {
    delete this.queries[query][queryId];
    if (Object.keys(this.queries[query]).length === 0) {
      delete this.queries[query];
    }
  }

  async getQuery(query: string): Promise<Query> {
    await this.setupPromise;

    // The same query may be launched multiple times,
    // a "sub" id is needed to differentiate them
    const queryId = this.nextId;
    this.nextId += 1;

    this.queries[query] = this.queries[query] || {};
    this.queries[query][queryId] = {
      promise: undefined,
      resolve: undefined,
    };

    this.queries[query][queryId].promise = new Promise((resolve) => {
      this.queries[query][queryId].resolve = resolve;

      this.worker.postMessage({ query, queryId });
    });

    const result: {
      searchedTerms: string[][],
      queryParts: QueryPart[],
    } = await this.queries[query][queryId].promise;

    const getNextN = async (n: number) => {
      if (!this.queries[query] || !this.queries[query][queryId]) {
        return []; // free() already called
      }

      await this.queries[query][queryId].promise;

      // Initiate worker request
      this.queries[query][queryId].promise = new Promise((resolve) => {
        this.queries[query][queryId].resolve = resolve;

        this.worker.postMessage({
          query, queryId, isGetNextN: true, n,
        });
      });

      if (!this.queries[query] || !this.queries[query][queryId]) {
        return []; // free() already called
      }

      // Wait for worker to finish
      const getNextNResult: {
        nextResults: [number, number][]
      } = await this.queries[query][queryId].promise;

      // Simple transform into Result objects
      const retrievedResults: Result[] = getNextNResult.nextResults.map(([docId, score]) => new Result(
        docId, score, this.config.fieldInfos,
      ));

      // Retrieve field stores
      await Promise.all(retrievedResults.map((res) => res.populate(
        this.options.url,
        this.cache,
        this.config,
      )));

      return retrievedResults;
    };

    const free = () => {
      this.deleteQuery(query, queryId);
      this.worker.postMessage({ query, isFree: true });
    };

    // eslint-disable-next-line consistent-return
    return new Query(
      query,
      result.searchedTerms,
      result.queryParts,
      getNextN,
      free,
    );
  }
}

export default Searcher;
