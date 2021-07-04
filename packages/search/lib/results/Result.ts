import { FieldInfo } from './FieldInfo';

class Result {
  storage: [number, string][] = Object.create(null);

  constructor(
    public docId: number,
    public score: number,
    private fieldInfos: FieldInfo[],
  ) {}

  async populate(baseUrl: string, fieldStoreBlockSize: number): Promise<void> {
    const fileNumber = Math.floor(this.docId / fieldStoreBlockSize);
    const fileUrl = `${baseUrl}/field_store/${fileNumber}.json`;
    try {
      this.storage = (await (await fetch(fileUrl, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      })).json())[this.docId % fieldStoreBlockSize];
    } catch (ex) {
      console.log(this.docId);
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
