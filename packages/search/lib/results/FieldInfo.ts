interface FieldInfo {
  [idOrName: string]: {
    id: number
    name: string,
    do_store: boolean,
    weight: number,
    k: number,
    b: number,
  }
}

export default FieldInfo;
