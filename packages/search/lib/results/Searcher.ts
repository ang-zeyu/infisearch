import Query from './Query';
import {
  FieldInfo, MorselsConfigRaw, MorselsConfig,
} from './FieldInfo';
import { SearcherOptions } from './SearcherOptions';
import Result from './Result';
import { QueryPart } from '../parser/queryParser';
import JsonCache from './JsonCache';

declare const MORSELS_VERSION;

class Searcher {
  morselsConfig: MorselsConfig;

  private readonly setupPromise: Promise<any>;

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

  private persistentJsonCache: JsonCache;

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

    this.setupPromise = this.retrieveConfig()
      .then(() => {
        options.useQueryTermProximity = options.useQueryTermProximity
            && this.morselsConfig.indexingConfig.withPositions;

        this.worker.postMessage(this.morselsConfig);
      })
      .then(() => this.cacheFieldStores())
      .then(() => workerSetup);
  }

  async cacheFieldStores() {
    if (!this.options.cacheAllFieldStores) {
      return;
    }

    this.persistentJsonCache = new JsonCache();
    const promises: [string, Promise<any>][] = [];

    const lastFileNumber = Math.ceil(this.morselsConfig.lastDocId / this.morselsConfig.fieldStoreBlockSize);
    const { numStoresPerDir, indexingConfig } = this.morselsConfig;
    const { numDocsPerBlock } = indexingConfig;
    for (let i = 0; i < lastFileNumber; i++) {
      const dirNumber = Math.floor(i / numStoresPerDir);
      const blockNumber = Math.floor(i / numDocsPerBlock);
      const url = `${this.options.url}field_store/${dirNumber}/${i}--${blockNumber}.json`;
      promises.push([url, fetch(url).then(res => res.json())]);

      // Throttle to 10 unresolved requests. A little arbitrary for now.
      if (promises.length >= 10) {
        // TODO make this non-sequential?
        // (first promise that resolved might not be the earliest, although likely)
        const first = promises.shift();
        this.persistentJsonCache.linkToJsons[first[0]] = await first[1];
      }
    }

    const jsons = await Promise.all(promises.map(p => p[1]));
    promises.forEach((val, idx) => {
      this.persistentJsonCache.linkToJsons[val[0]] = jsons[idx];
    });
  }

  async retrieveConfig(): Promise<void> {
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
      searchedTerms: string[],
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
      const jsonCache = this.persistentJsonCache || new JsonCache();
      await Promise.all(retrievedResults.map((res) => res.populate(
        this.options.url,
        jsonCache,
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
