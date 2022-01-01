export interface SearcherOptions {
  url: string,
  numberOfExpandedTerms?: number,
  useQueryTermProximity?: boolean,
  cacheAllFieldStores?: boolean,
  useWand?: number,
  resultLimit?: number,
}
