import { FieldInfo, MorselsConfig } from './Config';
import PersistentCache from './Cache';
import { getFieldUrl } from '../utils/FieldStore';

class Result {
  storage: [number, string][] = Object.create(null);

  constructor(
    public docId: number,
    public score: number,
    private fieldInfos: FieldInfo[],
  ) {}

  async populate(
    baseUrl: string,
    cache: PersistentCache,
    config: MorselsConfig,
  ): Promise<void> {
    const fileUrl = getFieldUrl(baseUrl, this.docId, config);
    try {
      const rawJson = await cache.getJson(fileUrl);

      let idx = this.docId % config.fieldStoreBlockSize;
      const { numDocsPerBlock } = config.indexingConfig;
      if (numDocsPerBlock < config.fieldStoreBlockSize) {
        idx %= numDocsPerBlock;
      }

      this.storage = rawJson[idx];
    } catch (ex) {
      console.log(ex);
    }
  }

  getSingleField(fieldName: string): string {
    const field = this.fieldInfos.find((fieldInfo) => fieldInfo.name === fieldName);
    const matchingPair = this.storage.find(([fieldId]) => fieldId === field.id);
    return matchingPair ? matchingPair[1] : '';
  }

  getStorageWithFieldNames(): [string, string][] {
    return this.storage.map(([fieldId, content]) => [this.fieldInfos[fieldId].name, content]);
  }
}

export default Result;
