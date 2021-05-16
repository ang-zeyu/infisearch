interface FieldInfo {
  [idOrName: string]: {
    id: number
    name: string,
    do_store: boolean,
    weight: number
  }
}

export default FieldInfo;
