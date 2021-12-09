import Query from './Query';
import {
  FieldInfo, MorselsConfigRaw, MorselsConfig,
} from './FieldInfo';
import { SearcherOptions } from './SearcherOptions';
import Result from './Result';
import { QueryPart } from '../parser/queryParser';
import TempJsonCache from './TempJsonCache';

declare const MORSELS_VERSION;

class Searcher {
  morselsConfig: MorselsConfig;

  private readonly setupPromise: Promise<any>;

  private worker: Worker;

  private workerQueryPromises: {
    [query: string]: {
      [timestamp: number]: {
        promise: Promise<any>,
        resolve: any,
      }
    }
  } = Object.create(null);

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
            timestamp,
            nextResults,
            searchedTerms,
            queryParts,
          } = ev.data;

          this.workerQueryPromises[query][timestamp].resolve({
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

    this.setupPromise = this.retrieveConfig().then(() => {
      options.useQueryTermProximity = options.useQueryTermProximity
          && this.morselsConfig.indexingConfig.withPositions;

      this.worker.postMessage(this.morselsConfig);

      return workerSetup;
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

    const { field_infos: fieldInfosRaw } = json;

    const fieldInfos: FieldInfo[] = [];
    Object.entries(fieldInfosRaw.field_infos_map).forEach(([fieldName, fieldInfo]) => {
      fieldInfo.name = fieldName;
      fieldInfos.push(fieldInfo as FieldInfo);
    });
    fieldInfos.sort((a, b) => a.id - b.id);

    this.morselsConfig = {
      indexingConfig: {
        loaderConfigs: json.indexing_config.loader_configs,
        plNamesToCache: json.indexing_config.pl_names_to_cache,
        numPlsPerDir: json.indexing_config.num_pls_per_dir,
        numStoresPerDir: json.indexing_config.num_stores_per_dir,
        withPositions: json.indexing_config.with_positions,
      },
      langConfig: json.lang_config,
      fieldInfos,
      numScoredFields: fieldInfosRaw.num_scored_fields,
      fieldStoreBlockSize: fieldInfosRaw.field_store_block_size,
      searcherOptions: this.options,
    };
  }

  private deleteQuery(query: string, timestamp: number) {
    delete this.workerQueryPromises[query][timestamp];
    if (Object.keys(this.workerQueryPromises[query]).length === 0) {
      delete this.workerQueryPromises[query];
    }
  }

  async getQuery(query: string): Promise<Query> {
    await this.setupPromise;

    const timestamp = new Date().getTime();

    this.workerQueryPromises[query] = this.workerQueryPromises[query] || {};
    this.workerQueryPromises[query][timestamp] = {
      promise: undefined,
      resolve: undefined,
    };

    this.workerQueryPromises[query][timestamp].promise = new Promise((resolve) => {
      this.workerQueryPromises[query][timestamp].resolve = resolve;

      this.worker.postMessage({ query, timestamp });
    });

    const result: {
      searchedTerms: string[],
      queryParts: QueryPart[],
    } = await this.workerQueryPromises[query][timestamp].promise;

    const getNextN = async (n: number) => {
      await this.workerQueryPromises[query][timestamp].promise;

      this.workerQueryPromises[query][timestamp].promise = new Promise((resolve) => {
        this.workerQueryPromises[query][timestamp].resolve = resolve;

        this.worker.postMessage({
          query, timestamp, isGetNextN: true, n,
        });
      });

      const getNextNResult: {
        nextResults: [number, number][]
      } = await this.workerQueryPromises[query][timestamp].promise;

      const retrievedResults = getNextNResult.nextResults.map(([docId, score]) => new Result(
        docId, score, this.morselsConfig.fieldInfos,
      ));

      const tempJsonCache = new TempJsonCache();
      await Promise.all(retrievedResults.map((res) => res.populate(
        this.options.url,
        tempJsonCache,
        this.morselsConfig.fieldStoreBlockSize,
        this.morselsConfig.indexingConfig.numStoresPerDir,
      )));

      return retrievedResults;
    };

    const free = () => {
      this.deleteQuery(query, timestamp);
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
