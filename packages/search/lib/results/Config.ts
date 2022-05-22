export interface SearcherOptions {
  url: string,
  numberOfExpandedTerms?: number,
  useQueryTermProximity?: boolean,
  cacheAllFieldStores?: boolean,
  resultLimit?: number,
}

export interface MorselsConfig {
  ver: string,
  indexVer: string,
  lastDocId: number,
  indexingConfig: {
    loaderConfigs: { [loader: string]: any },
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
  fieldStoreBlockSize: number,
  numStoresPerDir: number,
  // Added in Searcher.ts
  searcherOptions: SearcherOptions
}

export interface FieldInfo {
  id: number
  name: string,
  do_store: boolean,
  weight: number,
  k: number,
  b: number,
}
