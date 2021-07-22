import { Query, Searcher } from '../../../librarian_common/pkg';
import { FieldInfo } from '../results/FieldInfo';
import { SearcherOptions } from '../results/SearcherOptions';
import WorkerQuery from './workerQuery';

export interface WorkerSearcherSetup {
  url: string,
  fieldInfos: {
    numScoredFields: number,
    fieldInfos: FieldInfo[],
  },
  searcherOptions: SearcherOptions,
}

export default class WorkerSearcher {
  workerQueries: { [query: string]: WorkerQuery } = Object.create(null);

  wasmModule;

  wasmSearcher: Searcher;

  private baseUrl: string;

  private fieldStoreBlockSize: number;

  private numScoredFields: number;

  private fieldInfos: FieldInfo[];

  private searcherOptions: SearcherOptions;

  constructor(data: WorkerSearcherSetup) {
    this.baseUrl = data.url;
    this.numScoredFields = data.fieldInfos.numScoredFields;
    this.fieldInfos = data.fieldInfos.fieldInfos;
    this.searcherOptions = data.searcherOptions;
  }

  async processQuery(query: string): Promise<WorkerQuery> {
    this.freeQuery(query);

    const wasmQuery: Query = await this.wasmModule.get_query(this.wasmSearcher.get_ptr(), query);

    this.workerQueries[query] = new WorkerQuery(
      wasmQuery.get_aggregated_terms(),
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
    this.wasmModule = await import('../../../librarian_common/pkg');
    this.wasmSearcher = await this.wasmModule.get_new_searcher(
      this.baseUrl,
      this.numScoredFields,
      this.fieldInfos,
      {
        url: this.baseUrl,
        use_query_term_expansion: this.searcherOptions.useQueryTermExpansion,
        use_query_term_proximity: this.searcherOptions.useQueryTermProximity,
      },
    );
  }

  static async setup(data: WorkerSearcherSetup): Promise<WorkerSearcher> {
    const workerSearcher = new WorkerSearcher(data);

    await workerSearcher.setupWasm();

    return workerSearcher;
  }
}
