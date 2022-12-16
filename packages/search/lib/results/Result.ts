import { FieldInfo, InfiConfig } from './Config';
import PersistentCache from './Cache';
import { getFieldUrl } from '../utils/FieldStore';
import { linkHeadings } from './Result/linker';
import { Segment } from './Result/MatchResult';

interface EnumFields {
  [enumFieldName: string]: string | null
}

interface I64Fields {
  [numFieldName: string]: bigint | null
}

export class Result {
  constructor(
    public fields: {
      texts: [string, string][],
      enums: EnumFields,
      numbers: I64Fields,
    },
    private _mrlRegexes: RegExp[],
  ) {}

  static async _mrlPopulate(
    byteOffset: number,
    raw: DataView,
    regexes: RegExp[],
    baseUrl: string,
    cache: PersistentCache,
    cfg: InfiConfig,
    enumFieldInfos: FieldInfo[],
    i64FieldInfos: FieldInfo[],
  ): Promise<Result> {
    const docId = raw.getUint32(byteOffset, true);
    // eslint-disable-next-line no-param-reassign
    byteOffset += 4;

    // -------------------------------------
    // Retrieve and populate textual fields
    const fileUrl = getFieldUrl(baseUrl, docId, cfg);
    const rawJson: [string, string][][] = await cache.getJson(fileUrl);

    let idx = docId % cfg.numDocsPerStore;
    const { numDocsPerBlock } = cfg.indexingConfig;
    if (numDocsPerBlock < cfg.numDocsPerStore) {
      idx %= numDocsPerBlock;
    }

    const texts = rawJson[idx];
    // -------------------------------------

    // -------------------------------------
    // Populate enum, numeric fields
    const enums: EnumFields = {};
    for (const fi of enumFieldInfos) {
      const enumValue = raw.getUint8(byteOffset);
      enums[fi.name] = fi.enumInfo.enumValues[enumValue - 1] || null;

      // eslint-disable-next-line no-param-reassign
      byteOffset += 1;
    }

    const numbers: I64Fields = {};
    for (const fi of i64FieldInfos) {
      numbers[fi.name] = raw.getBigUint64(byteOffset, true);
      // eslint-disable-next-line no-param-reassign
      byteOffset += 8;
    }
    // -------------------------------------

    return new Result({ texts, enums, numbers }, regexes);
  }

  getHeadingBodyExcerpts(): Segment[] {
    return linkHeadings(this.fields.texts, this._mrlRegexes);
  }

  getKVFields(...fieldNames: string[]): { [fieldName: string]: string } {
    const numFields = fieldNames.length;
    const fieldsToPopulate = Object.create(null);
    let numFieldsEncountered = 0;

    for (const fieldNameAndField of this.fields.texts) {
      const [fieldName, fieldText] = fieldNameAndField;
      if (!(fieldName in fieldsToPopulate)) {
        fieldsToPopulate[fieldName] = fieldText;
        numFieldsEncountered += 1;

        if (numFieldsEncountered === numFields) {
          break;
        }
      }
    }

    return fieldsToPopulate;
  }
}
