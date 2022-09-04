export enum QueryPartType {
  TERM = 'TERM',
  PHRASE = 'PHRASE',
  BRACKET = 'BRACKET',
  AND = 'AND',
  NOT = 'NOT',
}

export interface QueryPart {
  isCorrected?: boolean;
  isStopWordRemoved?: boolean;
  autoSuffixWildcard: boolean;
  suffixWildcard: boolean;
  isSuffixed: boolean;
  originalTerms?: string[];
  partType: QueryPartType;
  terms?: string[];
  termsSearched?: string[][];
  children?: QueryPart[];
  weight?: number;
}
