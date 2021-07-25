import Query from './Query';
import { FieldInfosRaw, FieldInfo } from './FieldInfo';
import { SearcherOptions } from './SearcherOptions';
import Result from './Result';
import { WorkerSearcherSetup } from '../worker/workerSearcher';
import { QueryPart } from '../parser/queryParser';

class Searcher {
  private fieldStoreBlockSize: number;

  private numScoredFields: number;

  private fieldInfos: FieldInfo[];

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

    this.setupPromise = this.setupFieldInfo().then(() => {
      const message: WorkerSearcherSetup = {
        url: options.url,
        fieldInfos: {
          fieldInfos: this.fieldInfos,
          numScoredFields: this.numScoredFields,
        },
        searcherOptions: options,
      };
      this.worker.postMessage(message);

      return workerSetup;
    });
  }

  async setupFieldInfo(): Promise<void> {
    const json: FieldInfosRaw = await (await fetch(`${this.options.url}/fieldInfo.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    this.fieldStoreBlockSize = json.field_store_block_size;

    this.numScoredFields = json.num_scored_fields;

    this.fieldInfos = [];
    Object.entries(json.field_infos_map).forEach(([fieldName, fieldInfo]) => {
      fieldInfo.name = fieldName;
      this.fieldInfos.push(fieldInfo as FieldInfo);
    });
    this.fieldInfos.sort((a, b) => a.id - b.id);

    console.log(this.fieldInfos);
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
      const retrievedResults = nextDocIds.map((docId) => new Result(docId, 0, this.fieldInfos));
      await Promise.all(retrievedResults.map((res) => res.populate(
        this.options.url, this.fieldStoreBlockSize,
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
      this.fieldInfos,
      this.options,
      this.fieldStoreBlockSize,
      getNextN,
      free,
    );
  }
}

export default Searcher;
