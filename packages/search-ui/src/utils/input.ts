import { Query, Searcher } from '@morsels/search-lib';
import { Options } from '../Options';
import loadQueryResults from '../searchResultTransform';
import createElement, { createInvisibleLoadingIndicator } from './dom';

export class InputState {
  currQuery: Query;

  /**
   * Are there any results in the list container currently?
   */
  _mrlIsResultsBlank = true;

  _mrlIsRunningQuery = false;

  /**
   * An input will only take one action at a time, and queue only one.
   * This facilitates query pre-emption.
   */
  _mrlNextAction: () => any;

  _mrlLoader: HTMLElement = createInvisibleLoadingIndicator();

  _mrlLastElObserver: IntersectionObserver;
}

export async function runNewQuery(
  queryString: string,
  inputState: InputState,
  searcher: Searcher,
  root: HTMLElement,
  listContainer: HTMLElement,
  options: Options,
): Promise<void> {
  const { uiOptions } = options;
  inputState._mrlIsRunningQuery = true;

  const newIndicatorElement = uiOptions.loadingIndicatorRender(
    createElement, options, false, inputState._mrlIsResultsBlank,
  );
  inputState._mrlLoader.replaceWith(newIndicatorElement);
  inputState._mrlLoader = newIndicatorElement;

  try {
    // const now = performance.now();

    inputState.currQuery?.free();
    inputState.currQuery = await searcher.getQuery(queryString);

    // console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    const resultsDisplayed = await loadQueryResults(
      inputState, inputState.currQuery, searcher.cfg,
      true,
      listContainer,
      options,
    );
    if (resultsDisplayed) {
      inputState._mrlIsResultsBlank = false;
    }

    root.scrollTo({ top: 0 });
    listContainer.scrollTo({ top: 0 });
  } catch (ex) {
    listContainer.innerHTML = '';
    listContainer.appendChild(uiOptions.headerRender(createElement, options, true, false));
    throw ex;
  } finally {
    // Run the next queued query if there is one
    if (inputState._mrlNextAction) {
      const nextActionTemp = inputState._mrlNextAction;
      inputState._mrlNextAction = undefined;
      await nextActionTemp();
    } else {
      inputState._mrlIsRunningQuery = false;
    }
  }
}
