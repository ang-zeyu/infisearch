import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Query from './Query';
import { FieldInfosRaw, FieldInfo } from './FieldInfo';
import DocInfo from './DocInfo';
import parseQuery, { QueryPart, QueryPartType, Tokenizer } from '../parser/queryParser';
import preprocess from '../parser/queryPreprocessor';
import postprocess from '../parser/queryPostProcessor';
import { SearcherOptions } from './SearcherOptions';
import Result from './Result';
import { WorkerSearcherSetup } from '../worker/workerSearcher';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docInfo: DocInfo;

  private fieldStoreBlockSize: number;

  private numScoredFields: number;

  private fieldInfos: FieldInfo[];

  private readonly setupPromise: Promise<any>;

  private tokenizer: Tokenizer;

  private stopWords: Set<string>;

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

    // this.setupPromise = this.setup();
  }

  setupDocInfo(numScoredFields: number) {
    this.docInfo = new DocInfo(this.options.url, numScoredFields);
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

  async setup() {
    let wasmModule: any = import('../../../librarian_search/pkg/index.js');

    await this.setupFieldInfo();

    this.setupDocInfo(this.numScoredFields);
    await this.docInfo.initialisedPromise;

    this.dictionary = new Dictionary(this.options.workerUrl);
    await this.dictionary.setup(this.options.url, this.docInfo.numDocs);

    this.postingsListManager = new PostingsListManager(
      this.options.url,
      this.dictionary,
      this.fieldInfos,
      this.docInfo.numDocs,
    );

    wasmModule = await wasmModule;
    this.tokenizer = wasmModule.wasm_tokenize;
    this.stopWords = new Set(JSON.parse(wasmModule.get_stop_words()));
  }

  getAggregatedTerms(queryParts: QueryPart[], seen: Set<string>, result: string[]) {
    queryParts.forEach((queryPart) => {
      if (queryPart.terms) {
        if (queryPart.isStopWordRemoved) {
          result.push(queryPart.originalTerms[0]);
        }

        queryPart.terms.forEach((term) => {
          if (seen.has(term)) {
            return;
          }

          result.push(term);
        });
      } else if (queryPart.children) {
        this.getAggregatedTerms(queryPart.children, seen, result);
      }
    });
  }

  async getQuery(query: string): Promise<Query> {
    await this.setupPromise;

    const useWasm = true;
    if (useWasm) {
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
        [],
        undefined,
        undefined,
        this.fieldInfos,
        undefined,
        this.options,
        this.fieldStoreBlockSize,
        getNextN,
        free,
      );
    }

    // TODO tokenize by language
    const queryParts = parseQuery(query, this.tokenizer);
    // console.log(JSON.stringify(queryParts, null, 4));
    // const queryTerms: string[] = query.toLowerCase().split(/\s+/g);

    /* const queryVectors = queryParts
      .map((queryTerm, idx) => {
        this.dictionary.getTerms(queryTerm, idx === queryTerms.length - 1);
      })
      .filter((queryVec) => queryVec.getAllTerms().length);
    const aggregatedTerms = queryVectors.reduce((acc, queryVec) => acc.concat(queryVec.getAllTerms()), []);
    console.log(aggregatedTerms); */

    const isFreeTextQuery = queryParts.every((queryPart) => queryPart.type === QueryPartType.TERM);

    const preProcessedQueryParts = await preprocess(queryParts, isFreeTextQuery, this.stopWords, this.dictionary);
    console.log('preprocessed');
    console.log(JSON.stringify(preProcessedQueryParts, null, 4));

    const postingsLists = await this.postingsListManager.retrieveTopLevelPls(preProcessedQueryParts);
    console.log('processed');
    console.log(postingsLists);

    const postProcessedQueryParts = await postprocess(queryParts, postingsLists, this.dictionary,
      this.options);

    const aggregatedTerms: string[] = [];
    this.getAggregatedTerms(queryParts, new Set<string>(), aggregatedTerms);

    return new Query(
      query,
      aggregatedTerms,
      postProcessedQueryParts,
      postingsLists,
      isFreeTextQuery,
      this.docInfo,
      this.fieldInfos,
      this.dictionary,
      this.options,
      this.fieldStoreBlockSize,
    );
  }
}

export default Searcher;
