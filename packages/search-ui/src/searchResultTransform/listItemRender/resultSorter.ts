import { Segment } from '@infisearch/search-lib/lib/results/Result/MatchResult';
  
/**
 * Sorts the results according to some opinionated preferences,
 * then serializes them.
 */
export function sortAndLimitResults(matchResults: Segment[], maxSubMatches: number) {
  matchResults.sort((a, b) => {
    const termsA = a.numTerms;
    const termsB = b.numTerms;
    if (termsA === termsB) {
      // If there same terms matched for both matches, prefer "longer" snippets
      return b.type.localeCompare(a.type)
        || (b.text.length - a.text.length);
    }
    return termsB - termsA;
  });

  const maxMatches = Math.min(matchResults.length, maxSubMatches);
  let i = 0;
  for (; i < maxMatches; i += 1) {
    if (matchResults[i].numTerms !== matchResults[0].numTerms
      || matchResults[i].type !== matchResults[0].type) {
      break;
    }
  }

  matchResults.splice(i, matchResults.length);
}

