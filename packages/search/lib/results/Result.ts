import { FieldInfo } from './FieldInfo';

class Result {
  storage: [number, string][] = Object.create(null);

  constructor(
    public docId: number,
    public score: number,
    private fieldInfos: FieldInfo[],
  ) {}

  async populate(baseUrl: string, fieldStoreBlockSize: number, numStoresPerDir: number): Promise<void> {
    const fileNumber = Math.floor(this.docId / fieldStoreBlockSize);
    const dirNumber = Math.floor(fileNumber / numStoresPerDir);
    const fileUrl = `${baseUrl}field_store/${dirNumber}/${fileNumber}.json`;
    try {
      const rawJson = await (await fetch(fileUrl)).json();
      this.storage = rawJson[this.docId % fieldStoreBlockSize];
    } catch (ex) {
      console.log(ex);
    }
  }

  getSingleField(fieldName: string): string {
    const field = this.fieldInfos.find((fieldInfo) => fieldInfo.name === fieldName);
    if (!field) {
      return '';
    }

    const matchingPair: [number, string] = this.storage.find(
      (fieldIdContentPair) => fieldIdContentPair[0] === field.id,
    );
    return matchingPair ? matchingPair[1] : '';
  }

  getStorageWithFieldNames(): [string, string][] {
    return this.storage.map((fieldIdContentPair) => {
      const clonedPair = fieldIdContentPair.map((v) => v);
      clonedPair[0] = this.fieldInfos[clonedPair[0]].name;
      return clonedPair as [string, string];
    });
  }
}

export default Result;
