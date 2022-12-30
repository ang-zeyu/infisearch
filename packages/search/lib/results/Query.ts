import escapeStringRegexp from 'escape-string-regexp';

import { Result } from './Result';
import { QueryPart } from '../parser/queryParser';
import { InfiConfig } from './Config';


function getSearchedTerms(queryParts: QueryPart[], result: string[][], notContext: boolean) {
  for (const queryPart of queryParts) {
    const currNotContext = (queryPart.isSubtracted || queryPart.isInverted)
      ? !notContext
      : notContext;

    if (queryPart.termsSearched) {
      if (currNotContext) {
        result.push([...queryPart.termsSearched]);
      }
    } else if (queryPart.children) {
      getSearchedTerms(
        queryPart.children,
        result,
        currNotContext,
      );
    }
  }
}

export function getRegexes(queryParts: QueryPart[], config: InfiConfig) {
  const termRegexes: RegExp[] = [];

  const searchedTerms: string[][] = [];
  getSearchedTerms(queryParts, searchedTerms, true);

  const searchedTermsFlat: string[] = [];
  for (const innerTerms of searchedTerms) {
    const innerTermsJoined = innerTerms
      .map(t => {
        searchedTermsFlat.push(t);
        return escapeStringRegexp(t);
      })
      .sort((a, b) => b.length - a.length)
      .join('|');

    if (config.langConfig.lang === 'ascii_stemmer') {
      const nonEndBoundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})(\\w*?)(?=\\W|$)`, 'gi');
      termRegexes.push(nonEndBoundariedRegex);
    } else {
      const boundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})((?=\\W|$))`, 'gi');
      termRegexes.push(boundariedRegex);
    }
  }
  return [termRegexes, JSON.stringify(searchedTermsFlat)];
}

export default class Query {
  _mrlRegexes: RegExp[];


  constructor(
    /**
     * Original query string.
     */
    public readonly query: string,
    /**
     * Total number of results.
     */
    public readonly resultsTotal: number,
    /**
     * Syntactic tree of query parsed by InfiSearch.
     */
    public readonly queryParts: QueryPart[],
    /**
     * Returns the next N results.
     */
    public readonly getNextN: (n: number) => Promise<Result[]>,
    /**
     * Freeing a query manually is required since its results live in the WebWorker.
     */
    public readonly free: () => void,

    // Internal
    public readonly _mrlTermsFlattened: string,
  ) {}
}
