interface FieldInfo {
  [id: number]: {
    name: string,
    storage: string,
    storageParams: { baseName: string, [param: string]: any },
    weight: number
  }
}

export default FieldInfo;
