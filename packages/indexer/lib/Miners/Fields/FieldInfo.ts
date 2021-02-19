interface FieldInfo {
  [fieldName: string]: {
    id: number,
    storage: string,
    storageParams: { baseName: string, [param: string]: any },
    weight: number
  }
}

export default FieldInfo;
