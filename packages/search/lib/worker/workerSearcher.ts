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
      JSON.parse(wasmQuery.get_searched_terms()),
      JSON.parse(wasmQuery.get_query_parts()),
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

  private async _mrlSetupWasm(metadata: ArrayBuffer, wasmModule: Promise<any>) {
    this._mrlWasmModule = await wasmModule;

    const {
      indexingConfig,
      langConfig: { lang, options },
      fieldInfos,
      numScoredFields,
      searcherOptions,
    } = this._mrlConfig;

    const encoder = new TextEncoder();

    let stopWords: Uint8Array | undefined = undefined;

    const stopWordsOption: string[] | undefined = options.stop_words;
    if (stopWordsOption) {
      const encodedStopWords = stopWordsOption
        .map((sw) => encoder.encode(sw))
        .filter((swEncoded) => swEncoded.length < 255);
      const totalLength = encodedStopWords.length
        + encodedStopWords.reduce((acc, next) => acc + next.length, 0);

      // Stored in ... byteLength stopWordEncoded ... format
      stopWords = new Uint8Array(totalLength);

      let writePos = 0;
      encodedStopWords.forEach((encodedSw) => {
        stopWords[writePos++] = encodedSw.length;
        stopWords.set(encodedSw, writePos);
        writePos += encodedSw.length;
      });
    }

    const encodedFieldNames = fieldInfos.map((fieldInfo) => encoder.encode(fieldInfo.name));
    const fieldNameTotalLength = encodedFieldNames.reduce((acc, next) => acc + next.length, 0);

    const fieldInfosSerialized = new Uint8Array(
      /*
       "13" from:
       - 1 u8 to store each field name length
       - 4 bytes for f32 for each
         - weight
         - k
         - b
      */
      13 * encodedFieldNames.length + fieldNameTotalLength,
    );
    // Separate view to write floats then copy into fieldInfosSerialized
    const fieldInfosFloatsTemp = new Float32Array(3);

    let fieldInfosSerializedPos = 0;
    fieldInfos.forEach((fieldInfo, idx) => {
      const fieldNameByteLength = encodedFieldNames[idx].length;
      fieldInfosSerialized[fieldInfosSerializedPos++] = fieldNameByteLength;
      fieldInfosSerialized.set(encodedFieldNames[idx], fieldInfosSerializedPos);
      fieldInfosSerializedPos += fieldNameByteLength;

      fieldInfosFloatsTemp[0] = fieldInfo.weight;
      fieldInfosFloatsTemp[1] = fieldInfo.k;
      fieldInfosFloatsTemp[2] = fieldInfo.b;

      fieldInfosSerialized.set(new Uint8Array(fieldInfosFloatsTemp.buffer), fieldInfosSerializedPos);

      fieldInfosSerializedPos += 12;
    });

    this._mrlWasmSearcher = this._mrlWasmModule.get_new_searcher(
      metadata,
      indexingConfig.numPlsPerDir,
      indexingConfig.withPositions,
      lang,
      stopWords,
      options.stemmer,
      options.max_term_len,
      fieldInfosSerialized,
      numScoredFields,
      searcherOptions.url,
      searcherOptions.numberOfExpandedTerms,
      searcherOptions.useQueryTermProximity,
      searcherOptions.plLazyCacheThreshold,
      searcherOptions.resultLimit,
    );
  }

  static async _mrlSetup(data: MorselsConfig, wasmModule: Promise<any>): Promise<WorkerSearcher> {
    const workerSearcher = new WorkerSearcher(data);

    const baseUrl = data.searcherOptions.url;
    const metadataUrl = `${baseUrl}metadata.json`;

    let cache: Cache;
    try {
      cache = await caches.open(`morsels:${baseUrl}`);
    } catch {
      // Cache API blocked / unsupported (e.g. firefox private)
    }

    const metadata = await (
      cache
        ? cache.match(metadataUrl)
          .then((resp) => !resp && cache.add(metadataUrl))
          .then(() => cache.match(metadataUrl))
        : fetch(metadataUrl)
    ).then((resp) => resp.arrayBuffer());

    await workerSearcher._mrlSetupWasm(metadata, wasmModule);

    return workerSearcher;
  }
}
