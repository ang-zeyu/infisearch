import { Query } from '@infisearch/search-lib';
import { Result } from '@infisearch/search-lib/lib/results/Result';
import h from '@infisearch/search-lib/lib/utils/dom';
import { Options } from '../Options';
import { resultSeparator } from './resultsRender/repeatedFooter';

export async function resultsRender(
  options: Options,
  results: Result[],
  query: Query,
  numResultsSoFar: number,
  loadMore: (nResults: number) => Promise<HTMLElement[] | undefined>,
  focusOption: (el: HTMLElement) => void,
): Promise<HTMLElement[]> {
  const { resultsPerPage, listItemRender } = options.uiOptions;

  const resultEls = await Promise.all(results.map(
    (result) => listItemRender(h, options, result, query),
  ));

  resultEls.push(resultSeparator(
    options,
    numResultsSoFar + results.length,
    results.length < resultsPerPage,
    loadMore, focusOption, query,
  ));

  return resultEls;
}
