import Query from './Query';
import {
  FieldInfo, MorselsConfigRaw, MorselsConfig,
} from './FieldInfo';
import { SearcherOptions } from './SearcherOptions';
import Result from './Result';
import { QueryPart } from '../parser/queryParser';
import PersistentCache from './Cache';

declare const MORSELS_VERSION;

class Searcher {
  morselsConfig: MorselsConfig;

  isSetupDone: boolean = false;

  readonly setupPromise: Promise<any>;

  private worker: Worker;

  private workerQueryPromises: {
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

          this.workerQueryPromises[query][queryId].resolve({
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
      .then(() => {
        options.useQueryTermProximity = options.useQueryTermProximity
            && this.morselsConfig.indexingConfig.withPositions;

        this.worker.postMessage(this.morselsConfig);
      })
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
        if (this.morselsConfig.indexVer !== cacheIndexVer) {
          await caches.delete(cacheName);
          cache = await caches.open(cacheName);
        }
      }

      await cache.put('/index_ver', new Response(this.morselsConfig.indexVer));
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

    const lastFileNumber = Math.ceil(this.morselsConfig.lastDocId / this.morselsConfig.fieldStoreBlockSize);
    const { numStoresPerDir, indexingConfig } = this.morselsConfig;
    const { numDocsPerBlock } = indexingConfig;
    for (let i = 0; i < lastFileNumber; i++) {
      const dirNumber = Math.floor(i / numStoresPerDir);
      const lastDocIdOfFile = Math.min(
        this.morselsConfig.lastDocId,
        (i + 1) * this.morselsConfig.fieldStoreBlockSize,
      );

      for (
        let docId = i * this.morselsConfig.fieldStoreBlockSize;
        docId < lastDocIdOfFile; docId += numDocsPerBlock
      ) {
        const blockNumber = Math.floor(docId / numDocsPerBlock);
        const url = `${this.options.url}field_store/${dirNumber}/${i}--${blockNumber}.json`;
        this.cache.cacheJson(url);
      }
    }
  }

  private setupIndexCache() {
    const pls = this.morselsConfig.indexingConfig.plNamesToCache;
    pls.forEach((pl) => {
      const folder = Math.floor(pl / this.morselsConfig.indexingConfig.numPlsPerDir);
      const url = `${this.options.url}pl_${folder}/pl_${pl}.json`;
      this.cache.cacheUrl(url);
    });
  }

  private async retrieveConfig(): Promise<void> {
    const json: MorselsConfigRaw = await (await fetch(`${this.options.url}morsels_config.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    if (json.ver !== MORSELS_VERSION) {
      throw new Error('Morsels search version not equal to indexer version!');
    }

    if (!('cacheAllFieldStores' in this.options)) {
      this.options.cacheAllFieldStores = !!json.cache_all_field_stores;
    }

    const { field_infos: fieldInfosRaw } = json;

    const fieldInfos: FieldInfo[] = [];
    Object.entries(fieldInfosRaw.field_infos_map).forEach(([fieldName, fieldInfo]) => {
      fieldInfo.name = fieldName;
      fieldInfos.push(fieldInfo as FieldInfo);
    });
    fieldInfos.sort((a, b) => a.id - b.id);

    this.morselsConfig = {
      indexVer: json.index_ver,
      lastDocId: json.last_doc_id,
      indexingConfig: {
        loaderConfigs: json.indexing_config.loader_configs,
        plNamesToCache: json.indexing_config.pl_names_to_cache,
        numDocsPerBlock: json.indexing_config.num_docs_per_block,
        numPlsPerDir: json.indexing_config.num_pls_per_dir,
        withPositions: json.indexing_config.with_positions,
      },
      langConfig: json.lang_config,
      fieldInfos,
      numScoredFields: fieldInfosRaw.num_scored_fields,
      fieldStoreBlockSize: fieldInfosRaw.field_store_block_size,
      numStoresPerDir: fieldInfosRaw.num_stores_per_dir,
      searcherOptions: this.options,
    };
  }

  private deleteQuery(query: string, queryId: number) {
    delete this.workerQueryPromises[query][queryId];
    if (Object.keys(this.workerQueryPromises[query]).length === 0) {
      delete this.workerQueryPromises[query];
    }
  }

  async getQuery(query: string): Promise<Query> {
    await this.setupPromise;

    // The same query may be launched multiple times,
    // a "sub" id is needed to differentiate them
    const queryId = this.nextId;
    this.nextId += 1;

    this.workerQueryPromises[query] = this.workerQueryPromises[query] || {};
    this.workerQueryPromises[query][queryId] = {
      promise: undefined,
      resolve: undefined,
    };

    this.workerQueryPromises[query][queryId].promise = new Promise((resolve) => {
      this.workerQueryPromises[query][queryId].resolve = resolve;

      this.worker.postMessage({ query, queryId });
    });

    const result: {
      searchedTerms: string[][],
      queryParts: QueryPart[],
    } = await this.workerQueryPromises[query][queryId].promise;

    const getNextN = async (n: number) => {
      if (!this.workerQueryPromises[query] || !this.workerQueryPromises[query][queryId]) {
        return []; // free() already called
      }

      await this.workerQueryPromises[query][queryId].promise;

      // Initiate worker request
      this.workerQueryPromises[query][queryId].promise = new Promise((resolve) => {
        this.workerQueryPromises[query][queryId].resolve = resolve;

        this.worker.postMessage({
          query, queryId, isGetNextN: true, n,
        });
      });

      if (!this.workerQueryPromises[query] || !this.workerQueryPromises[query][queryId]) {
        return []; // free() already called
      }

      // Wait for worker to finish
      const getNextNResult: {
        nextResults: [number, number][]
      } = await this.workerQueryPromises[query][queryId].promise;

      // Simple transform into Result objects
      const retrievedResults: Result[] = getNextNResult.nextResults.map(([docId, score]) => new Result(
        docId, score, this.morselsConfig.fieldInfos,
      ));

      // Retrieve field stores
      await Promise.all(retrievedResults.map((res) => res.populate(
        this.options.url,
        this.cache,
        this.morselsConfig,
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
