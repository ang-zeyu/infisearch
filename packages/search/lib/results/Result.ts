import { FieldInfo, MorselsConfig } from './Config';
import PersistentCache from './Cache';
import { getFieldUrl } from '../utils/FieldStore';
import { linkHeadings } from './Result/linker';
import { Segment } from './Result/MatchResult';

interface EnumFields {
  [enumFieldName: string]: string | null
}

export class Result {
  constructor(
    public fields: {
      texts: [string, string][],
      enums: EnumFields,
    },
    private _mrlRegexes: RegExp[],
  ) {}

  static async _mrlPopulate(
    offset: number,
    raw: Uint32Array,
    regexes: RegExp[],
    baseUrl: string,
    cache: PersistentCache,
    cfg: MorselsConfig,
    enumFieldInfos: FieldInfo[],
  ): Promise<Result> {
    const docId = raw[offset];

    // -------------------------------------
    // Retrieve and populate textual fields
    const fileUrl = getFieldUrl(baseUrl, docId, cfg);
    const rawJson = await cache.getJson(fileUrl);

    let idx = docId % cfg.numDocsPerStore;
    const { numDocsPerBlock } = cfg.indexingConfig;
    if (numDocsPerBlock < cfg.numDocsPerStore) {
      idx %= numDocsPerBlock;
    }

    const texts = rawJson[idx]
      .map(([fieldId, content]) => [cfg.fieldInfos[fieldId].name, content]);
    // -------------------------------------

    // -------------------------------------
    // Populate enum fields
    const enums: EnumFields = {};
    for (const fi of enumFieldInfos) {
      // eslint-disable-next-line no-param-reassign
      offset += 1;

      const enumValue = raw[offset];
      enums[fi.name] = fi.enumInfo.enumValues[enumValue - 1] || null;
    }
    // -------------------------------------

    return new Result({ texts, enums }, regexes);
  }

  getHeadingBodyExcerpts(): Segment[] {
    return linkHeadings(this.fields.texts, this._mrlRegexes);
  }
  
  getKVFields(fieldsToPopulate: { [fieldName: string]: null | string }) {
    const numFields = Object.keys(fieldsToPopulate).length;
    let numFieldsEncountered = 0;

    for (const fieldNameAndField of this.fields.texts) {
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
