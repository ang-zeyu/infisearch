export enum QueryPartType {
  TERM = 'TERM',
  PHRASE = 'PHRASE',
  BRACKET = 'BRACKET',
}

export interface QueryPart {
  isMandatory: boolean,
  isSubtracted: boolean,
  isInverted: boolean,
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
