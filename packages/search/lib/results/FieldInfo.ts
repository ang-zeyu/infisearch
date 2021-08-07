import { SearcherOptions } from './SearcherOptions';

export interface LibrarianConfig {
  indexingConfig: {
    withPositions: boolean,
    plNamesToCache: number[],
  },
  language: {
    lang: string,
    options: any,
  },
  fieldInfos: FieldInfo[],
  numScoredFields: number,
  fieldStoreBlockSize: number,
  searcherOptions: SearcherOptions
}

export interface LibrarianConfigRaw {
  indexing_config: {
    with_positions: boolean,
    pl_names_to_cache: number[],
  },
  language: {
    lang: string,
    options: any,
  },
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
}
