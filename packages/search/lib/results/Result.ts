import { FieldInfo, MorselsConfig } from './Config';
import PersistentCache from './Cache';
import { getFieldUrl } from '../utils/FieldStore';

class Result {
  private _mrlStorage: [number, string][] = Object.create(null);

  constructor(
    private _mrlDocId: number,
    public _mrlScore: number,
    private _mrlFieldInfos: FieldInfo[],
  ) {}

  async _mrlPopulate(
    baseUrl: string,
    cache: PersistentCache,
    config: MorselsConfig,
  ): Promise<void> {
    const fileUrl = getFieldUrl(baseUrl, this._mrlDocId, config);
    try {
      const rawJson = await cache.getJson(fileUrl);

      let idx = this._mrlDocId % config.fieldStoreBlockSize;
      const { numDocsPerBlock } = config.indexingConfig;
      if (numDocsPerBlock < config.fieldStoreBlockSize) {
        idx %= numDocsPerBlock;
      }

      this._mrlStorage = rawJson[idx];
    } catch (ex) {
      console.log(ex);
    }
  }

  getFields(): [string, string][] {
    return this._mrlStorage.map(([fieldId, content]) => [this._mrlFieldInfos[fieldId].name, content]);
  }
}

export default Result;
