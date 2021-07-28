export interface LibrarianConfig {
  language: {
    lang: string,
  },
  fieldInfos: FieldInfo[],
  numScoredFields: number,
  fieldStoreBlockSize: number,
}

export interface LibrarianConfigRaw {
  language: {
    lang: string,
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
