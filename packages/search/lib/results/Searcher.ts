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
      promise: Promise<any>,
      resolve: any,
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
            nextDocIds,
            aggregatedTerms,
            queryParts,
          } = ev.data;

          this.workerQueryPromises[query].resolve({
            query,
            nextDocIds,
            aggregatedTerms,
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
      },
      language: json.language,
      fieldInfos,
      numScoredFields: fieldInfosRaw.num_scored_fields,
      fieldStoreBlockSize: fieldInfosRaw.field_store_block_size,
    };
  }

  async getQuery(query: string): Promise<Query> {
    await this.setupPromise;

    const promise: Promise<{
      aggregatedTerms: string[],
      queryParts: QueryPart[],
    }> = new Promise(async (resolve) => {
      if (this.workerQueryPromises[query]) {
        await this.workerQueryPromises[query].promise;
      }

      this.workerQueryPromises[query] = { promise, resolve };
      this.worker.postMessage({ query });
    });

    const result: {
      aggregatedTerms: string[],
      queryParts: QueryPart[],
    } = await promise;
    delete this.workerQueryPromises[query];
    console.log(result);
    if (!result) {
      console.error('Worker error promise resolved with undefined');
      return;
    }

    const getNextN = async (n: number) => {
      if (this.workerQueryPromises[query]) {
        await this.workerQueryPromises[query].promise;
      }
      const getNextNPromise: Promise<{ nextDocIds: number[] }> = new Promise((resolve) => {
        this.workerQueryPromises[query] = { promise: getNextNPromise, resolve };
        this.worker.postMessage({ query, isGetNextN: true, n });
      });
      const getNextNResult: { nextDocIds: number[] } = await getNextNPromise;
      if (!getNextNResult) {
        console.error('Worker error promise resolved with undefined');
        return [];
      }

      const { nextDocIds } = getNextNResult;

      // console.log(retrievedResults);
      const retrievedResults = nextDocIds.map((docId) => new Result(
        docId, 0, this.librarianConfig.fieldInfos,
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
      result.aggregatedTerms,
      result.queryParts,
      this.options,
      getNextN,
      free,
    );
  }
}

export default Searcher;
