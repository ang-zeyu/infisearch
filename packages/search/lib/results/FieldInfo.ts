export interface FieldInfo {
  id: number
  name: string,
  do_store: boolean,
  weight: number,
  k: number,
  b: number,
}

export interface FieldInfosRaw {
  field_infos_map: { [fieldName: string]: FieldInfo },
  num_scored_fields: number,
  field_store_block_size: number,
}
