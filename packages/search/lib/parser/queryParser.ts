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
  shouldExpand?: boolean;
  isExpanded?: boolean;
  originalTerms?: string[];
  partType: QueryPartType;
  terms?: string[];
  children?: QueryPart[];
  weight?: number;
}
