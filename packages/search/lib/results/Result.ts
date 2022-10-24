import { FieldInfo, MorselsConfig } from './Config';
import PersistentCache from './Cache';
import { getFieldUrl } from '../utils/FieldStore';
import { linkHeadings } from './Result/linker';
import { Segment } from './Result/MatchResult';

export class Result {
  public fields: [string, string][] = [];

  constructor(
    private _mrlDocId: number,
    private _mrlFieldInfos: FieldInfo[],
    private _mrlRegexes: RegExp[],
  ) {}

  async _mrlPopulate(
    baseUrl: string,
    cache: PersistentCache,
    config: MorselsConfig,
  ): Promise<void> {
    const fileUrl = getFieldUrl(baseUrl, this._mrlDocId, config);
    try {
      const rawJson = await cache.getJson(fileUrl);

      let idx = this._mrlDocId % config.numDocsPerStore;
      const { numDocsPerBlock } = config.indexingConfig;
      if (numDocsPerBlock < config.numDocsPerStore) {
        idx %= numDocsPerBlock;
      }

      this.fields = rawJson[idx]
        .map(([fieldId, content]) => [this._mrlFieldInfos[fieldId].name, content]);
    } catch (ex) {
      console.log(ex);
    }
  }

  getHeadingBodyExcerpts(): Segment[] {
    return linkHeadings(this.fields, this._mrlRegexes);
  }
  
  getKVFields(fieldsToPopulate: { [fieldName: string]: null | string }) {
    const numFields = Object.keys(fieldsToPopulate).length;
    let numFieldsEncountered = 0;

    for (const fieldNameAndField of this.fields) {
      const [fieldName, fieldText] = fieldNameAndField;
      if (fieldsToPopulate[fieldName] === null) {
        fieldsToPopulate[fieldName] = fieldText;
        numFieldsEncountered += 1;
      }

      if (numFieldsEncountered === numFields) {
        break;
      }
    }
  }
}
