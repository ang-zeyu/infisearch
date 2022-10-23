import { Query, Searcher } from '@morsels/search-lib';
import h from '@morsels/search-lib/lib/utils/dom';
import { Options } from '../Options';
import loadQueryResults from '../searchResultTransform';
import { createInvisibleLoadingIndicator } from './dom';
import { addKeyboardHandler } from './keyboard';

export class InputState {
  _mrlCurrQuery: Query;

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

  constructor(
    public readonly _mrlInputEl: HTMLInputElement,
    public readonly _mrlListContainer: HTMLElement,
  ) {
    addKeyboardHandler(_mrlInputEl, _mrlListContainer);
  }
}

export async function runNewQuery(
  queryString: string,
  inputState: InputState,
  searcher: Searcher,
  root: HTMLElement,
  listContainer: HTMLElement,
  options: Options,
): Promise<void> {
  const { loadingIndicatorRender, headerRender, resultsPerPage } = options.uiOptions;
  inputState._mrlIsRunningQuery = true;

  const newIndicatorElement = loadingIndicatorRender(
    h, options, false, inputState._mrlIsResultsBlank,
  );
  inputState._mrlLoader.replaceWith(newIndicatorElement);
  inputState._mrlLoader = newIndicatorElement;

  try {
    // const now = performance.now();

    inputState._mrlCurrQuery?.free();
    inputState._mrlCurrQuery = await searcher.runQuery(queryString);

    // console.log(`runQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    const resultsDisplayed = await loadQueryResults(
      searcher,
      inputState, inputState._mrlCurrQuery,
      resultsPerPage,
      0,
      options,
    );
    if (resultsDisplayed) {
      inputState._mrlIsResultsBlank = false;
    }

    root.scrollTo({ top: 0 });
    listContainer.scrollTo({ top: 0 });
  } catch (ex) {
    listContainer.innerHTML = '';
    listContainer.appendChild(headerRender(h, options, true, false));
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
