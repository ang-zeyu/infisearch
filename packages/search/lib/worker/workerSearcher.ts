import { LibrarianConfig } from '../results/FieldInfo';
import { SearcherOptions } from '../results/SearcherOptions';
import WorkerQuery from './workerQuery';

export interface WorkerSearcherSetup {
  url: string,
  config: LibrarianConfig,
  searcherOptions: SearcherOptions,
}

export default class WorkerSearcher {
  workerQueries: { [query: string]: WorkerQuery } = Object.create(null);

  wasmModule;

  wasmSearcher: any;

  private baseUrl: string;

  private config: LibrarianConfig;

  private searcherOptions: SearcherOptions;

  constructor(data: WorkerSearcherSetup) {
    this.baseUrl = data.url;
    this.config = data.config;
    this.searcherOptions = data.searcherOptions;
  }

  async processQuery(query: string): Promise<WorkerQuery> {
    this.freeQuery(query);

    const wasmQuery: any = await this.wasmModule.get_query(this.wasmSearcher.get_ptr(), query);

    this.workerQueries[query] = new WorkerQuery(
      wasmQuery.get_searched_terms(),
      wasmQuery.get_query_parts(),
      wasmQuery,
    );

    return this.workerQueries[query];
  }

  getQueryNextN(query: string, n: number): number[] {
    return this.workerQueries[query].getNextN(n);
  }

  freeQuery(query: string) {
    if (this.workerQueries[query]) {
      this.workerQueries[query].free();
    }
    delete this.workerQueries[query];
  }

  private async setupWasm() {
    const language = this.config.language.lang;
    this.wasmModule = await import(
      /* webpackChunkName: "librarian_search_wasm.[index]" */
      `../../../librarian_search/pkg/lang_${language}/index.js`
    );
    this.wasmSearcher = await this.wasmModule.get_new_searcher(
      this.baseUrl,
      this.config.numScoredFields,
      this.config.fieldInfos,
      this.config.indexingConfig,
      this.config.language, this.searcherOptions,
    );
  }

  static async setup(data: WorkerSearcherSetup): Promise<WorkerSearcher> {
    const workerSearcher = new WorkerSearcher(data);

    await workerSearcher.setupWasm();

    return workerSearcher;
  }
}
