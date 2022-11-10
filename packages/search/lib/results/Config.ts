export interface SearcherOptions {
  url: string,
  maxSuffixSearchTerms?: number,
  maxAutoSuffixSearchTerms?: number,
  useQueryTermProximity?: boolean,
  cacheAllFieldStores?: boolean,
  plLazyCacheThreshold: number,
  resultLimit?: number,
}

export function prepareSearcherOptions(searcherOptions: SearcherOptions) {
  if (!('url' in searcherOptions)) {
    throw new Error('Mandatory url parameter not specified');
  } else if (!searcherOptions.url.endsWith('/')) {
    searcherOptions.url += '/';
  }

  if (searcherOptions.url.startsWith('/')) {
    searcherOptions.url = window.location.origin + searcherOptions.url;
  }

  if (!('maxAutoSuffixSearchTerms' in searcherOptions)) {
    searcherOptions.maxAutoSuffixSearchTerms = 3;
  }

  if (!('maxSuffixSearchTerms' in searcherOptions)) {
    searcherOptions.maxSuffixSearchTerms = 5;
  }

  if (!('useQueryTermProximity' in searcherOptions)) {
    searcherOptions.useQueryTermProximity = true;
  }

  if (!('plLazyCacheThreshold' in searcherOptions)) {
    searcherOptions.plLazyCacheThreshold = 0;
  }

  if (!('resultLimit' in searcherOptions)) {
    searcherOptions.resultLimit = null;
  }
}

export interface MorselsConfig {
  ver: string,
  indexVer: string,
  lastDocId: number,
  indexingConfig: {
    loaders: { [loader: string]: any },
    plNamesToCache: number[],
    numDocsPerBlock: number,
    numPlsPerDir: number,
    withPositions: boolean,
  },
  langConfig: {
    lang: string,
    options: any,
  },
  cacheAllFieldStores: boolean,
  fieldInfos: FieldInfo[],
  numScoredFields: number,
  numDocsPerStore: number,
  numStoresPerDir: number,
  // Added in Searcher.ts
  searcherOptions: SearcherOptions
}

export interface FieldInfo {
  id: number
  name: string,
  enumInfo?: { enumId: number, enumValues: string[] }
  storeText: boolean,
  weight: number,
  k: number,
  b: number,
}
