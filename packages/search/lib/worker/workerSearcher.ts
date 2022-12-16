import { InfiConfig } from '../results/Config';
import { QueryOpts } from '../results/Searcher/QueryOpts';
import { serializeGetQueryParams } from '../utils/wasmParams';
import WorkerQuery from './workerQuery';

const workerQueries: {
  [queryId: number]: WorkerQuery
} = Object.create(null);

let wasmModule: any;

let wasmSearcher: any;

let config: InfiConfig;


export async function processQuery(
  query: string,
  opts: QueryOpts,
  queryId: number,
): Promise<WorkerQuery> {
  const wasmQuery: any = await wasmModule.get_query(
    wasmSearcher.get_ptr(), serializeGetQueryParams(query, opts, config),
  );

  const queryPartsRaw = wasmQuery.get_query_parts() as string;
  let queryParts = [];
  try {
    queryParts = JSON.parse(queryPartsRaw);
  } catch (ex) {
    console.error(`Error deserializing query parts:\n${queryPartsRaw}\n${ex}`);
  }

  workerQueries[queryId] = new WorkerQuery(
    queryParts,
    wasmQuery.results_total,
    wasmQuery,
  );

  return workerQueries[queryId];
}

export function getQueryNextN(queryId: number, n: number): ArrayBuffer {
  return (workerQueries[queryId]?._mrlGetNextN(n)) || new ArrayBuffer(0);
}

export function freeQuery(queryId: number) {
  if (workerQueries[queryId]) {
    workerQueries[queryId]._mrlFree();
    delete workerQueries[queryId];
  }
}


async function setupMetadata(baseUrl: string, innerUrl: string): Promise<ArrayBuffer> {
  let cache: Cache;
  try {
    cache = await caches.open(`infi:${baseUrl}`);
  } catch {
    // Cache API blocked / unsupported (e.g. firefox private)
  }

  const metadataUrl = `${innerUrl}/metadata.json`;

  return (
    cache
      ? cache.match(metadataUrl)
        .then((resp) => !resp && cache.add(metadataUrl))
        .then(() => cache.match(metadataUrl))
        .catch(() => fetch(metadataUrl))
      : fetch(metadataUrl)
  ).then((resp) => resp.arrayBuffer());
}

export async function setupWasm(
  cfg: InfiConfig,
  wasmModulePromise: Promise<any>,
) {
  config = cfg;

  const {
    indexVer,
    indexingConfig,
    langConfig: { lang, options },
    fieldInfos,
    numScoredFields,
    searcherOptions,
  } = config;

  const innerUrl = `${searcherOptions.url}${indexVer}/`;
  const metadataPromise = setupMetadata(searcherOptions.url, innerUrl);

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

  wasmModule = await wasmModulePromise;
  wasmSearcher = wasmModule.get_new_searcher(
    await metadataPromise,
    indexingConfig.numPlsPerDir,
    indexingConfig.withPositions,
    lang,
    stopWords,
    options.ignore_stop_words,
    options.stemmer,
    options.max_term_len,
    fieldInfosSerialized,
    numScoredFields,
    searcherOptions.url,
    innerUrl,
    searcherOptions.maxAutoSuffixSearchTerms,
    searcherOptions.maxSuffixSearchTerms,
    searcherOptions.useQueryTermProximity,
    searcherOptions.plLazyCacheThreshold,
    searcherOptions.resultLimit,
  );
}
