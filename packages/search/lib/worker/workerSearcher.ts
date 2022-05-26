import { MorselsConfig } from '../results/Config';
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

  private async setupWasm(metadataDictString: [ArrayBuffer, ArrayBuffer], wasmModule: Promise<any>) {
    const [metadata, dictString] = metadataDictString;
    this.wasmModule = await wasmModule;
    this.wasmSearcher = await this.wasmModule.get_new_searcher(this.config, metadata, dictString);
  }

  static async setup(data: MorselsConfig, wasmModule: Promise<any>): Promise<WorkerSearcher> {
    const workerSearcher = new WorkerSearcher(data);

    const baseUrl = data.searcherOptions.url;
    const metadataUrl = `${baseUrl}bitmap_docinfo_dicttable.json`;
    const dictStringUrl = `${baseUrl}dictionary_string.json`;

    let cache: Cache;
    try {
      cache = await caches.open(`morsels:${baseUrl}`);
    } catch {
      // Cache API blocked / unsupported (e.g. firefox private)
    }

    const metadataDictString = await Promise.all([
      (cache
        ? cache.match(metadataUrl)
          .then((resp) => !resp && cache.add(metadataUrl))
          .then(() => cache.match(metadataUrl))
        : fetch(metadataUrl)
      ).then((resp) => resp.arrayBuffer()),
      (cache
        ? cache.match(dictStringUrl)
          .then((resp) => !resp && cache.add(dictStringUrl))
          .then(() => cache.match(dictStringUrl))
        : fetch(dictStringUrl)
      ).then((resp) => resp.arrayBuffer()),
    ]);

    await workerSearcher.setupWasm(metadataDictString, wasmModule);

    return workerSearcher;
  }
}
