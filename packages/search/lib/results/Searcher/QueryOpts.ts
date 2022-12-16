export interface QueryOpts {
  enumFilters?: { [enumFieldName: string]: (string | null)[] },
  i64Filters?: { [numFieldName: string]: { gte?: number | bigint, lte?: number | bigint, } },
  sort?: string | null,
  sortAscending?: boolean,
}
