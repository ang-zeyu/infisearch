// Serialization of parameters from JS side to WASM

import { InfiConfig } from '../results/Config';
import { QueryOpts } from '../results/Searcher/QueryOpts';

let encoder = new TextEncoder();

export function serializeGetQueryParams(query: string, queryOpts: QueryOpts, config: InfiConfig): Uint8Array {
  const { enumFilters, i64Filters, sort, sortAscending } = queryOpts;
  const { fieldInfos } = config;

  /*
   Static parts:
   4 (query length)
   1 (enum filter count)
   1 (i64 filter count)
   1 + 1 (sortParam - i8 i8, first byte is a boolean indicator)
   1 (sortAscending)

   Dynamic parts:
   query's encoded length
   enumFilters.length * (
    1 (enumId)
    1 (number of enum values to filter for)
    N (number of enum values to filter for * 1 for the enum value id)
   )
   i64Filters.length * (
    1 (i64Id)
    1 (boolean - is there a lower bound?)
    8 (lower bound)
    1 (boolean - is there a upper bound?)
    8 (upper bound)
   )
  */


  const enumFiltersArr = Object.entries(enumFilters);
  const i64FiltersArr = Object.entries(i64Filters);

  const encodedQuery = encoder.encode(query);
  const queryLength = encodedQuery.length;
  const enumFilterEnumValuesParamLength = Object.values(enumFilters).reduce((acc, b) => acc + b.length, 0);

  const view = new DataView(new ArrayBuffer(
    9 + queryLength
      + enumFiltersArr.length * 2
      + enumFilterEnumValuesParamLength
      + i64FiltersArr.length * 20,
  ));

  let viewIdx = 4;

  function pushByte(byte: number) {
    view.setUint8(viewIdx, byte);
    viewIdx += 1;
  }

  function pushBigInt(n : number | bigint) {
    view.setBigInt64(viewIdx, BigInt(n), true);
    viewIdx += 8;
  }

  // ------------------------------------
  // Query
  view.setUint32(0, queryLength, true);
  encodedQuery.forEach(pushByte);

  // ------------------------------------
  // Enums

  const enumFilterCountIdx = viewIdx;
  view.setUint8(enumFilterCountIdx, 0);
  viewIdx += 1;

  enumFiltersArr.forEach(([fieldName, allowedEnumValues]) => {
    const fieldInfo = fieldInfos.find((fi) => fi.name === fieldName);
    if (fieldInfo) {
      const { enumId, enumValues } = fieldInfo.enumInfo;

      pushByte(enumId);
  
      const enumValuesFiltered = allowedEnumValues
        .filter((enumValue) => enumValue === null || enumValues.includes(enumValue))
        .map((enumValue) => enumValue === null
          ? 0
          // +1 as 0 is the "default" enum value
          : enumValues.findIndex((ev) => ev === enumValue) + 1,
        );

      pushByte(enumValuesFiltered.length);
      enumValuesFiltered.forEach(pushByte);

      view.setUint8(enumFilterCountIdx, view.getUint8(enumFilterCountIdx) + 1);
    }
  });

  // ------------------------------------
  // I64 Min Max filters

  const i64FilterCountIdx = viewIdx;
  view.setUint8(i64FilterCountIdx, 0);
  viewIdx += 1;

  i64FiltersArr.forEach(([fieldName, { gte, lte }]) => {
    const fieldInfo = fieldInfos.find((fi) => fi.name === fieldName);
    if (fieldInfo) {
      pushByte(fieldInfo.i64Info.id);
      const hasGte = gte !== undefined;
      pushByte(hasGte ? 1 : 0);
      if (hasGte) pushBigInt(gte);
      const hasLte = lte !== undefined;
      pushByte(hasLte ? 1 : 0);
      if (hasLte) pushBigInt(lte);

      view.setUint8(i64FilterCountIdx, view.getUint8(i64FilterCountIdx) + 1);
    }
  });

  // ------------------------------------
  // Sort parameters

  const sortField = fieldInfos.find((fi) => fi.name === sort);
  if (sortField) {
    pushByte(1);
    pushByte(sortField.i64Info.id);
  } else {
    pushByte(0);
  }

  pushByte(sortAscending ? 1 : 0);

  return new Uint8Array(view.buffer);
}
