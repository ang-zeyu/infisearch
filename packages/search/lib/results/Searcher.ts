import Query from './Query';
import { SearcherOptions, MorselsConfig } from './Config';
import Result from './Result';
import { QueryPart } from '../parser/queryParser';
import PersistentCache from './Cache';
import { getFieldUrl } from '../utils/FieldStore';

declare const MORSELS_VERSION;

// Code from
/* webpack/runtime/publicPath */
// manually handled since the WebWorker url is dynamic (based on language)
// TODO maybe require the worker URL be specified instead
let scriptUrl: string;
if (document.currentScript) {
  scriptUrl = (document.currentScript as HTMLScriptElement).src;
} else {
  const scripts = document.getElementsByTagName('script');
  scriptUrl = scripts.length && scripts[scripts.length - 1].src;
}
scriptUrl = scriptUrl.replace(/#.*$/, '').replace(/\?.*$/, '').replace(/\/[^\/]+$/, '/');

export const workerScript = { s: '' };

class Searcher {
  cfg: MorselsConfig;

  isSetupDone: boolean = false;

  readonly setupPromise: Promise<any>;

  private _mrlWorker: Worker;

  private _mrlQueries: {
    [queryId: number]: {
      promise: Promise<any>,
      resolve: any,
    }
  } = Object.create(null);

  // Use an auto-incrementing id to resolve queries to-fro the Worker
  private id = 0;

  private _mrlCache: PersistentCache;

  constructor(private _mrlOptions: SearcherOptions) {
    const configSetupPromise = this._mrlRetrieveConfig();
    this.setupPromise = Promise.all([
      configSetupPromise,
      new Promise<void>((resolve) => {
        const objectUrl = URL.createObjectURL(new Blob([
          `const __morsWrkrUrl="${scriptUrl}";const __indexUrl="${_mrlOptions.url}";${workerScript.s}`,
        ], { type: 'text/javascript' }));

        this._mrlWorker = new Worker(objectUrl);

        const cacheSetupPromise = this._mrlSetupCache(`morsels:${_mrlOptions.url}`);
      
        this._mrlWorker.onmessage = (ev) => {
          if (ev.data.query) {
            const {
              query,
              queryId,
              nextResults,
              searchedTerms,
              queryParts,
            } = ev.data;

            this._mrlQueries[queryId].resolve({
              query,
              nextResults,
              searchedTerms,
              queryParts,
            });
          } else if (ev.data === '') {
            Promise.all([
              configSetupPromise, cacheSetupPromise,
            ]).then(() => this._mrlWorker.postMessage(this.cfg));
            URL.revokeObjectURL(objectUrl);
          } else if (ev.data.isSetupDone) {
            this.isSetupDone = true;
            resolve();
            this._mrlSetupFieldStoreCache();
            this._mrlSetupIndexCache();
          }
        };

        this._mrlWorker.onmessageerror = (ev) => {
          console.log(ev);
        };
      }),
    ]);
  }

  private async _mrlSetupCache(cacheName: string) {
    try {
      const { indexVer } = this.cfg;

      let cache = await caches.open(cacheName);
      const cacheIndexVerResp = await cache.match('/index_ver');
      if (cacheIndexVerResp) {
        const cacheIndexVer = await cacheIndexVerResp.text();
        if (indexVer !== cacheIndexVer) {
          await caches.delete(cacheName);
          cache = await caches.open(cacheName);
        }
      }

      await cache.put('/index_ver', new Response(indexVer));
      this._mrlCache = new PersistentCache(cache);
    } catch {
      // Cache API blocked / unsupported (e.g. firefox private)
      this._mrlCache = new PersistentCache(undefined);
    }
  }

  private _mrlSetupFieldStoreCache() {
    if (!this._mrlOptions.cacheAllFieldStores) {
      return;
    }

    // These 2 parameters are "clean" multiples / divisors of each other
    const { fieldStoreBlockSize, indexingConfig, lastDocId } = this.cfg;
    const increment = Math.min(fieldStoreBlockSize, indexingConfig.numDocsPerBlock);
    for (let docId = 0; docId < lastDocId; docId += increment) {
      this._mrlCache._mrlCacheJson(getFieldUrl(this._mrlOptions.url, docId, this.cfg));
    }
  }

  private _mrlSetupIndexCache() {
    const pls = this.cfg.indexingConfig.plNamesToCache;
    pls.forEach((pl) => {
      const folder = Math.floor(pl / this.cfg.indexingConfig.numPlsPerDir);
      const url = `${this._mrlOptions.url}pl_${folder}/pl_${pl}.json`;
      this._mrlCache._mrlCacheUrl(url);
    });
  }

  private async _mrlRetrieveConfig(): Promise<void> {
    const searcherOpts = this._mrlOptions;
    this.cfg = await (await fetch(`${searcherOpts.url}morsels_config.json`)).json();

    if (this.cfg.ver !== MORSELS_VERSION) {
      throw new Error('Morsels search !== indexer version!');
    }

    if (!('cacheAllFieldStores' in searcherOpts)) {
      searcherOpts.cacheAllFieldStores = !!this.cfg.cacheAllFieldStores;
    }

    searcherOpts.useQueryTermProximity = searcherOpts.useQueryTermProximity
        && this.cfg.indexingConfig.withPositions;

    this.cfg.searcherOptions = searcherOpts;
  }

  async getQuery(query: string): Promise<Query> {
    await this.setupPromise;

    const queryId = this.id;
    this.id += 1;

    const queries = this._mrlQueries;

    queries[queryId] = {
      promise: undefined,
      resolve: undefined,
    };

    queries[queryId].promise = new Promise((resolve) => {
      queries[queryId].resolve = resolve;

      // Resolved when the worker replies
      this._mrlWorker.postMessage({ query, queryId });
    });

    const result: {
      searchedTerms: string[][],
      queryParts: QueryPart[],
    } = await queries[queryId].promise;

    const getNextN = async (n: number) => {
      if (!queries[queryId]) {
        return []; // free() already called
      }

      await queries[queryId].promise;

      // Initiate worker request
      queries[queryId].promise = new Promise((resolve) => {
        queries[queryId].resolve = resolve;

        // Resolved when the worker replies
        this._mrlWorker.postMessage({
          query, queryId, isGetNextN: true, n,
        });
      });

      if (!queries[queryId]) {
        return []; // free() already called
      }

      // Wait for worker to finish
      const getNextNResult: {
        nextResults: number[]
      } = await queries[queryId].promise;

      // Simple transform into Result objects
      const retrievedResults: Result[] = getNextNResult.nextResults.map((docId) => new Result(
        docId, this.cfg.fieldInfos,
      ));

      // Retrieve field stores
      await Promise.all(retrievedResults.map((res) => res._mrlPopulate(
        this._mrlOptions.url,
        this._mrlCache,
        this.cfg,
      )));

      return retrievedResults;
    };

    const free = () => {
      delete queries[queryId];
      this._mrlWorker.postMessage({ query, isFree: true });
    };

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
