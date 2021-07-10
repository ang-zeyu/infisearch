// Term expansion / etc.
import { QueryPart, QueryPartType } from './queryParser';
import { PostingsList, TermPostingsList } from '../PostingsList/PostingsList';
import Dictionary from '../Dictionary/Dictionary';
import { SearcherOptions } from '../results/SearcherOptions';

export default async function postprocess(
  queryParts: QueryPart[],
  postingsLists: PostingsList[],
  dictionary: Dictionary,
  options: SearcherOptions,
) : Promise<QueryPart[]> {
  const lastQueryPart = queryParts[queryParts.length - 1];
  if (
    options.useQueryTermExpansion
    && lastQueryPart
    && lastQueryPart.type === QueryPartType.TERM
    && lastQueryPart.shouldExpand
    && !lastQueryPart.isCorrected // don't expand spelling corrected terms
  ) {
    // Expand
    lastQueryPart.originalTerms = lastQueryPart.originalTerms || lastQueryPart.terms.map((t) => t);

    const expandedTerms = await dictionary.getExpandedTerms(lastQueryPart.terms[0]);

    lastQueryPart.isExpanded = !!Object.keys(expandedTerms).length;

    const extraLists = await Promise.all(
      Object.entries(expandedTerms)
        .map(async ([term, weight]) => {
          const termInfo = await dictionary.getTermInfo(term);

          const pl = new TermPostingsList(term, termInfo);
          pl.includeInProximityRanking = false;
          pl.weight = weight;

          await pl.fetch(options.url);

          return pl;
        }),
    );

    lastQueryPart.terms = Object.keys(expandedTerms).concat([lastQueryPart.terms[0]]);

    postingsLists.push(...extraLists);
  }

  return queryParts;
}
