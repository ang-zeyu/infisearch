import Query from './Query';
import {
  FieldInfo, LibrarianConfigRaw, LibrarianConfig,
} from './FieldInfo';
import { SearcherOptions } from './SearcherOptions';
import Result from './Result';
import { WorkerSearcherSetup } from '../worker/workerSearcher';
import { QueryPart } from '../parser/queryParser';

class Searcher {
  private librarianConfig: LibrarianConfig;

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
    this.worker = new Worker(options.workerUrl);

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
          && this.librarianConfig.indexingConfig.withPositions;

      const message: WorkerSearcherSetup = {
        url: options.url,
        config: this.librarianConfig,
        searcherOptions: options,
      };
      this.worker.postMessage(message);

      return workerSetup;
    });
  }

  async retrieveConfig(): Promise<void> {
    const json: LibrarianConfigRaw = await (await fetch(`${this.options.url}/_librarian_config.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    const { field_infos: fieldInfosRaw } = json;

    const fieldInfos: FieldInfo[] = [];
    Object.entries(fieldInfosRaw.field_infos_map).forEach(([fieldName, fieldInfo]) => {
      fieldInfo.name = fieldName;
      fieldInfos.push(fieldInfo as FieldInfo);
    });
    fieldInfos.sort((a, b) => a.id - b.id);

    console.log(fieldInfos);

    this.librarianConfig = {
      indexingConfig: {
        withPositions: json.indexing_config.with_positions,
        plNamesToCache: json.indexing_config.pl_names_to_cache,
      },
      language: json.language,
      fieldInfos,
      numScoredFields: fieldInfosRaw.num_scored_fields,
      fieldStoreBlockSize: fieldInfosRaw.field_store_block_size,
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

    const promise: Promise<{
      searchedTerms: string[],
      queryParts: QueryPart[],
    }> = new Promise((resolve) => {
      this.workerQueryPromises[query] = this.workerQueryPromises[query] || {};
      this.workerQueryPromises[query][timestamp] = {
        promise,
        resolve,
      };

      this.worker.postMessage({ query, timestamp });
    });

    const result: {
      searchedTerms: string[],
      queryParts: QueryPart[],
    } = await promise;
    this.deleteQuery(query, timestamp);

    console.log(result);
    if (!result) {
      console.error('Worker error promise resolved with undefined');
      return;
    }

    const getNextN = async (n: number) => {
      if (this.workerQueryPromises[query] && this.workerQueryPromises[query][timestamp]) {
        await this.workerQueryPromises[query][timestamp].promise;
      }

      const getNextNPromise: Promise<{ nextResults: [number, number][] }> = new Promise((resolve) => {
        this.workerQueryPromises[query] = this.workerQueryPromises[query] || {};
        this.workerQueryPromises[query][timestamp] = {
          promise: getNextNPromise,
          resolve,
        };

        this.worker.postMessage({
          query, timestamp, isGetNextN: true, n,
        });
      });

      const getNextNResult: { nextResults: [number, number][] } = await getNextNPromise;
      this.deleteQuery(query, timestamp);
      if (!getNextNResult) {
        console.error('Worker error promise resolved with undefined');
        return [];
      }

      const retrievedResults = getNextNResult.nextResults.map(([docId, score]) => new Result(
        docId, score, this.librarianConfig.fieldInfos,
      ));

      await Promise.all(retrievedResults.map((res) => res.populate(
        this.options.url, this.librarianConfig.fieldStoreBlockSize,
      )));

      return retrievedResults;
    };

    const free = () => {
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
