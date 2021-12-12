import { FieldInfo, MorselsConfig } from './FieldInfo';
import TempJsonCache from './TempJsonCache';

class Result {
  storage: [number, string][] = Object.create(null);

  constructor(
    public docId: number,
    public score: number,
    private fieldInfos: FieldInfo[],
  ) {}

  async populate(
    baseUrl: string,
    tempJsonCache: TempJsonCache,
    morselsConfig: MorselsConfig,
  ): Promise<void> {
    const { fieldStoreBlockSize, indexingConfig } = morselsConfig;
    const { numStoresPerDir, numDocsPerBlock } = indexingConfig;
    const fileNumber = Math.floor(this.docId / fieldStoreBlockSize);
    const blockNumber = Math.floor(this.docId / numDocsPerBlock);
    const dirNumber = Math.floor(fileNumber / numStoresPerDir);
    const fileUrl = `${baseUrl}field_store/${dirNumber}/${fileNumber}--${blockNumber}.json`;
    try {
      const rawJson = await tempJsonCache.fetch(fileUrl);

      let idx = this.docId % fieldStoreBlockSize;
      if (numDocsPerBlock < fieldStoreBlockSize) {
        idx %= numDocsPerBlock;
      }

      this.storage = rawJson[idx];
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
