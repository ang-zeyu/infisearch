import { MorselsConfig } from '../results/Config';
import WorkerQuery from './workerQuery';

export default class WorkerSearcher {
  private _mrlWorkerQueries: {
    [query: string]: {
      [queryId: number]: WorkerQuery
    }
  } = Object.create(null);

  private _mrlWasmModule: any;

  private _mrlWasmSearcher: any;

  constructor(private _mrlConfig: MorselsConfig) {}

  async _mrlProcessQuery(query: string, queryId: number): Promise<WorkerQuery> {
    const wasmQuery: any = await this._mrlWasmModule.get_query(this._mrlWasmSearcher.get_ptr(), query);

    this._mrlWorkerQueries[query] = this._mrlWorkerQueries[query] || {};
    this._mrlWorkerQueries[query][queryId] = new WorkerQuery(
      wasmQuery.get_searched_terms(),
      wasmQuery.get_query_parts(),
      wasmQuery,
    );

    return this._mrlWorkerQueries[query][queryId];
  }

  _mrlGetQueryNextN(query: string, queryId: number, n: number): number[] {
    return this._mrlWorkerQueries[query][queryId]._mrlGetNextN(n);
  }

  _mrlFreeQuery(query: string, queryId: number) {
    if (this._mrlWorkerQueries[query][queryId]) {
      this._mrlWorkerQueries[query][queryId]._mrlFree();
    }
    delete this._mrlWorkerQueries[query][queryId];
    if (Object.keys(this._mrlWorkerQueries[query]).length === 0) {
      delete this._mrlWorkerQueries[query];
    }
  }

  private async _mrlSetupWasm(metadataDictString: [ArrayBuffer, ArrayBuffer], wasmModule: Promise<any>) {
    const [metadata, dictString] = metadataDictString;
    this._mrlWasmModule = await wasmModule;
    this._mrlWasmSearcher = await this._mrlWasmModule.get_new_searcher(this._mrlConfig, metadata, dictString);
  }

  static async _mrlSetup(data: MorselsConfig, wasmModule: Promise<any>): Promise<WorkerSearcher> {
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

    await workerSearcher._mrlSetupWasm(metadataDictString, wasmModule);

    return workerSearcher;
  }
}
