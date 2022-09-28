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
  autoSuffixWildcard: boolean;
  suffixWildcard: boolean;
  isSuffixed: boolean;
  originalTerm?: string;
  partType: QueryPartType;
  term?: string;
  termsSearched?: string[];
  children?: QueryPart[];
  weight?: number;
}
