import { MorselsConfig } from '../results/FieldInfo';
import WorkerQuery from './workerQuery';

export default class WorkerSearcher {
  workerQueries: {
    [query: string]: {
      [queryId: number]: WorkerQuery
    }
  } = Object.create(null);

  wasmModule: any;

  wasmSearcher: any;

  constructor(private config: MorselsConfig) {}

  async processQuery(query: string, queryId: number): Promise<WorkerQuery> {
    const wasmQuery: any = await this.wasmModule.get_query(this.wasmSearcher.get_ptr(), query);

    this.workerQueries[query] = this.workerQueries[query] || {};
    this.workerQueries[query][queryId] = new WorkerQuery(
      wasmQuery.get_searched_terms(),
      wasmQuery.get_query_parts(),
      wasmQuery,
    );

    return this.workerQueries[query][queryId];
  }

  getQueryNextN(query: string, queryId: number, n: number): number[] {
    return this.workerQueries[query][queryId].getNextN(n);
  }

  freeQuery(query: string, queryId: number) {
    if (this.workerQueries[query][queryId]) {
      this.workerQueries[query][queryId].free();
    }
    delete this.workerQueries[query][queryId];
    if (Object.keys(this.workerQueries[query]).length === 0) {
      delete this.workerQueries[query];
    }
  }

  private async setupWasm() {
    const language = this.config.langConfig.lang;
    this.wasmModule = await import(
      /* webpackChunkName: "wasm.[request]" */
      `@morsels/lang-${language}/index.js`
    );
    this.wasmSearcher = await this.wasmModule.get_new_searcher(this.config);
  }

  static async setup(data: MorselsConfig): Promise<WorkerSearcher> {
    const workerSearcher = new WorkerSearcher(data);

    await workerSearcher.setupWasm();

    return workerSearcher;
  }
}
