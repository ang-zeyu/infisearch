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
    [query: string]: {
      [queryId: number]: {
        promise: Promise<any>,
        resolve: any,
      }
    }
  } = Object.create(null);

  private _mrlNextId = 0;

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

            this._mrlQueries[query][queryId].resolve({
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
      let cache = await caches.open(cacheName);
      const cacheIndexVerResp = await cache.match('/index_ver');
      if (cacheIndexVerResp) {
        const cacheIndexVer = await cacheIndexVerResp.text();
        if (this.cfg.indexVer !== cacheIndexVer) {
          await caches.delete(cacheName);
          cache = await caches.open(cacheName);
        }
      }

      await cache.put('/index_ver', new Response(this.cfg.indexVer));
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
    const { fieldStoreBlockSize, indexingConfig } = this.cfg;
    const increment = Math.min(fieldStoreBlockSize, indexingConfig.numDocsPerBlock);
    for (let docId = 0; docId < this.cfg.lastDocId; docId += increment) {
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
    this.cfg = await (await fetch(`${this._mrlOptions.url}morsels_config.json`)).json();

    if (this.cfg.ver !== MORSELS_VERSION) {
      throw new Error('Morsels search !== indexer version!');
    }

    if (!('cacheAllFieldStores' in this._mrlOptions)) {
      this._mrlOptions.cacheAllFieldStores = !!this.cfg.cacheAllFieldStores;
    }

    this._mrlOptions.useQueryTermProximity = this._mrlOptions.useQueryTermProximity
        && this.cfg.indexingConfig.withPositions;

    this.cfg.searcherOptions = this._mrlOptions;
  }

  private _mrlDeleteQuery(query: string, queryId: number) {
    delete this._mrlQueries[query][queryId];
    if (Object.keys(this._mrlQueries[query]).length === 0) {
      delete this._mrlQueries[query];
    }
  }

  async getQuery(query: string): Promise<Query> {
    await this.setupPromise;

    // The same query may be launched multiple times,
    // a "sub" id is needed to differentiate them
    const queryId = this._mrlNextId;
    this._mrlNextId += 1;

    this._mrlQueries[query] = this._mrlQueries[query] || {};
    this._mrlQueries[query][queryId] = {
      promise: undefined,
      resolve: undefined,
    };

    this._mrlQueries[query][queryId].promise = new Promise((resolve) => {
      this._mrlQueries[query][queryId].resolve = resolve;

      this._mrlWorker.postMessage({ query, queryId });
    });

    const result: {
      searchedTerms: string[][],
      queryParts: QueryPart[],
    } = await this._mrlQueries[query][queryId].promise;

    const getNextN = async (n: number) => {
      if (!this._mrlQueries[query] || !this._mrlQueries[query][queryId]) {
        return []; // free() already called
      }

      await this._mrlQueries[query][queryId].promise;

      // Initiate worker request
      this._mrlQueries[query][queryId].promise = new Promise((resolve) => {
        this._mrlQueries[query][queryId].resolve = resolve;

        this._mrlWorker.postMessage({
          query, queryId, isGetNextN: true, n,
        });
      });

      if (!this._mrlQueries[query] || !this._mrlQueries[query][queryId]) {
        return []; // free() already called
      }

      // Wait for worker to finish
      const getNextNResult: {
        nextResults: number[]
      } = await this._mrlQueries[query][queryId].promise;

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
      this._mrlDeleteQuery(query, queryId);
      this._mrlWorker.postMessage({ query, isFree: true });
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
