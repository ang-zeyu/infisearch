import { Query, Searcher } from '@morsels/search-lib';
import h from '@morsels/search-lib/lib/utils/dom';
import { Options } from './Options';
import { resultsRender } from './searchResultTransform/resultsRender';
import { createInvisibleLoadingIndicator } from './utils/dom';
import { InputState } from './utils/input';
import { focusEl } from './utils/keyboard';


/**
 * @returns The rendered result elements, or undefined if pre-emptively disrupted by a new query
 */
export default async function loadQueryResults(
  searcher: Searcher,
  inputState: InputState,
  query: Query,
  resultsToLoad: number,
  numResultsSoFar: number,
  options: Options,
): Promise<HTMLElement[] | undefined> {
  // If a new query interrupts the current one
  if (inputState._mrlNextAction) return;

  // let now = performance.now();

  const results = await query.getNextN(resultsToLoad);

  // console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);

  if (inputState._mrlNextAction) return;

  // now = performance.now();

  const inputEl = inputState._mrlInputEl;
  const listContainer = inputState._mrlListContainer;
  const resultsEls = await resultsRender(
    options,
    results,
    query,
    numResultsSoFar,
    (nResults: number) => {
      // inputEl.focus(); -- this wont work. causes keyboard to reshow on mobile
      return loadQueryResults(
        searcher, inputState, query, 
        nResults, numResultsSoFar + results.length, options,
      );
    },
    (el: HTMLElement) => focusEl(
      el, listContainer.querySelector('#morsels-list-selected'), inputEl, listContainer, false,
    ),
  );

  // console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  if (inputState._mrlNextAction) return;

  if (numResultsSoFar) {
    listContainer.append(...resultsEls);
  } else {
    listContainer.innerHTML = '';
    inputState._mrlLoader = createInvisibleLoadingIndicator();
    listContainer.append(
      inputState._mrlLoader,
      options.uiOptions.headerRender(h, options, false, false, query),
      ...resultsEls,
    );
  }

  return resultsEls;
}
