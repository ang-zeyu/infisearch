import { QueryPart, QueryPartType } from './queryParser';
import Dictionary from '../Dictionary/Dictionary';

// Deal with non-existent terms, spelling errors
export default async function preprocess(
  queryParts: QueryPart[],
  isFreeTextQuery: boolean,
  stopWords: Set<string>,
  dictionary: Dictionary,
) : Promise<QueryPart[]> {
  const promises: Promise<any>[] = [];

  for (let i = 0; i < queryParts.length; i += 1) {
    const queryPart = queryParts[i];
    if (queryPart.terms) {
      for (let j = 0; j < queryPart.terms.length; j += 1) {
        const term = queryPart.terms[j];
        if (isFreeTextQuery && stopWords.has(term)) {
          queryPart.terms.splice(j, 1);
          j -= 1;
          continue;
        }

        const termInfo = await dictionary.getTermInfo(term);
        if (!termInfo) {
          queryPart.isCorrected = true;
          queryPart.originalTerms = queryPart.originalTerms || queryPart.terms.map((t) => t);

          const correctedTerm = await dictionary.getBestCorrectedTerm(term);
          if (correctedTerm) {
            queryPart.terms[j] = correctedTerm;
          } else {
            queryPart.terms.splice(j, 1);
            j -= 1;
          }
        }
      }

      if (!queryPart.terms.length) {
        queryParts.splice(i, 1);
        i -= 1;
      }
    } else if (queryPart.children) {
      promises.push(preprocess(queryPart.children, isFreeTextQuery, stopWords, dictionary));
    }
  }

  await Promise.all(promises);

  return queryParts;
}
