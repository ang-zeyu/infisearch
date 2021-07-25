export enum QueryPartType {
  TERM = 'TERM',
  PHRASE = 'PHRASE',
  BRACKET = 'BRACKET',
  AND = 'AND',
  NOT = 'NOT',
  ADDED = 'ADDED',
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
