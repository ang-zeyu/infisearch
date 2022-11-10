import { Query, Searcher } from '@infisearch/search-lib';
import { Options } from './Options';
import { resultsRender } from './searchResultTransform/resultsRender';
import { IManager } from './InputManager';
import { focusEl, SELECTED_OPTION_ID } from './utils/keyboard';


/**
 * @returns The rendered result elements, or undefined if pre-emptively disrupted by a new query
 */
export default async function loadQueryResults(
  searcher: Searcher,
  iManager: IManager,
  query: Query,
  resultsToLoad: number,
  numResultsSoFar: number,
  options: Options,
): Promise<HTMLElement[] | undefined> {
  // If a new query interrupts the current one
  if (iManager._mrlHasQueuedAction()) return;

  const results = await query.getNextN(resultsToLoad);

  if (iManager._mrlHasQueuedAction()) return;

  // const now = performance.now();

  const inputEl = iManager._mrlInputEl;
  const resultContainer = iManager._mrlScroller.children[3] as HTMLElement;
  const resultsEls = await resultsRender(
    options,
    results,
    query,
    numResultsSoFar,
    (nResults: number) => {
      // inputEl.focus(); -- this wont work. causes keyboard to reshow on mobile
      return loadQueryResults(
        searcher, iManager, query, 
        nResults, numResultsSoFar + results.length, options,
      );
    },
    (el: HTMLElement) => focusEl(
      el,
      resultContainer.querySelector(`#${SELECTED_OPTION_ID}`),
      inputEl,
      resultContainer,
      false,
    ),
  );

  // console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  // We could pre-empt at this stage too, but it is likely not beneficial
  // since it takes very less time to complete...
  // if (iManager._mrlHasQueuedAction()) return;

  if (!numResultsSoFar) {
    iManager._mrlRefreshLoader();
    iManager._mrlRefreshHeader(query);
    resultContainer.innerHTML = '';
  }
  resultContainer.append(...resultsEls);

  return resultsEls;
}
