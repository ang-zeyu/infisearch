import { SearcherOptions } from './SearcherOptions';

export interface MorselsConfig {
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
  fieldInfos: FieldInfo[],
  numScoredFields: number,
  fieldStoreBlockSize: number,
  numStoresPerDir: number,
  searcherOptions: SearcherOptions
}

export interface MorselsConfigRaw {
  ver: string,
  last_doc_id: number,
  indexing_config: {
    loader_configs: { [loader: string]: any },
    pl_names_to_cache: number[],
    num_docs_per_block: number,
    num_pls_per_dir: number,
    with_positions: boolean,
  },
  lang_config: {
    lang: string,
    options: any,
  },
  cache_all_field_stores: boolean,
  field_infos: FieldInfosRaw,
}

export interface FieldInfo {
  id: number
  name: string,
  do_store: boolean,
  weight: number,
  k: number,
  b: number,
}

interface FieldInfosRaw {
  field_infos_map: { [fieldName: string]: FieldInfo },
  num_scored_fields: number,
  field_store_block_size: number,
  num_stores_per_dir: number,
}
