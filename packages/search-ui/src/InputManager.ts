import { Query, Searcher } from '@infisearch/search-lib';
import { Options } from './Options';
import { MultiSelectState, filtersRender } from './search/multiSelectFilters';
import createTipButton from './search/tips';
import loadQueryResults from './searchResultTransform';
import { headerRender } from './utils/header';
import { setExpanded, unsetActiveDescendant, unsetExpanded } from './utils/aria';
import { addKeyboardHandler as addKeyboardHandlers } from './utils/keyboard';
import { stateRender } from './utils/state';

const INPUT_HAS_STRING_CLASS = 'morsels-empty-input';

/**
 * An IManager can be running one action, and queueing only one at a time.
 * When the current action is done, it will "pop" the existing one.
 * 
 * It should also be able to "reset" instantaneously.
 * 
 * This facilitates query pre-emption.
 */
export class IManager {
  /**
   * Current Query object this input is managing. Must be free()-ed as appropriate.
   */
  private _mrlCurrQuery: Query;

  /**
   * An input will only take one action at a time, and queue only one.
   * This facilitates query pre-emption.
   */
  private _mrlNextAction: () => Promise<any>;

  private _mrlIsRunningAction = false;

  private _mrlFiltersStates: MultiSelectState[] = [];

  /**
   * Supporting elements
   */

  private _mrlHeader: HTMLElement;

  private _mrlFilters: HTMLElement;

  private _mrlGetOrSetFiltersShown: (setValue?: boolean) => boolean;

  private _mrlState: HTMLElement;

  private _mrlResultContainer: HTMLElement;

  constructor(
    public readonly _mrlInputEl: HTMLInputElement,
    public readonly _mrlSearcher: Searcher,
    public readonly _mrlScroller: HTMLElement,
    public readonly _mrlOptions: Options,
  ) {
    const that = this;
    const listContainerChildren = _mrlScroller.children;
    that._mrlHeader = listContainerChildren[0] as HTMLElement;
    that._mrlFilters = listContainerChildren[1] as HTMLElement;
    that._mrlState = listContainerChildren[2] as HTMLElement;
    that._mrlResultContainer = listContainerChildren[3] as HTMLElement;

    addKeyboardHandlers(_mrlInputEl, that._mrlResultContainer);
    that._mrlRefreshEmptyInputClass();
    that._mrlRefreshHeader();

    that._mrlRunOrQueue(async () => {
      await _mrlSearcher.setupPromise;

      // ----------------------------------------------------------------
      // Setup the category filters, which needs the cfg file
      // from the Searcher instance initialised
      const cfg = _mrlSearcher.cfg;
      const [controls, states, getOrSetFiltersShown] = filtersRender(_mrlOptions, cfg, that);
      that._mrlFiltersStates = states;
      controls.append(createTipButton(_mrlOptions.uiOptions, cfg));
      that._mrlFilters.replaceWith(controls);
      that._mrlFilters = controls;
      that._mrlGetOrSetFiltersShown = getOrSetFiltersShown;
      that._mrlRefreshHeader();
      // ----------------------------------------------------------------

    });
  }

  /**
   * Facilitates pre-emption.
   */
  _mrlHasQueuedAction() {
    return !!this._mrlNextAction;
  }

  _mrlRefreshLoader(isDone = true, isError = false) {
    const that = this;

    const isResultsContainerEmpty = !that._mrlResultContainer.childElementCount;
    const newIndicatorElement = stateRender(
      !that._mrlSearcher.isSetupDone,
      isResultsContainerEmpty,
      !that._mrlInputEl.value,
      isDone,
      isError,
    );

    that._mrlState.replaceWith(newIndicatorElement);
    that._mrlState = newIndicatorElement;
  }

  _mrlRefreshHeader(query?: Query) {
    const el = headerRender(query, this._mrlGetOrSetFiltersShown);
    this._mrlHeader.replaceWith(el);
    this._mrlHeader = el;
  }

  private _mrlRefreshEmptyInputClass() {
    if (this._mrlInputEl.value.length) {
      this._mrlScroller.classList.remove(INPUT_HAS_STRING_CLASS);
    } else {
      this._mrlScroller.classList.add(INPUT_HAS_STRING_CLASS);
    }
  }

  /**
   * Called when the input is blank.
   * Resets should pre-empt everything (be instantaneous),
   * and resolve ongoing actions correctly.
   */
  _mrlReset() {
    const that = this;
    // -----------------------------------
    // Update the DOM
    that._mrlRefreshEmptyInputClass();
    that._mrlRefreshLoader();
    that._mrlRefreshHeader();
    that._mrlResultContainer.innerHTML = '';
    unsetActiveDescendant(that._mrlInputEl);
    unsetExpanded(that._mrlInputEl);
    // -----------------------------------

    // -----------------------------------
    // Manage queued/running actions properly
    if (that._mrlIsRunningAction) {
      /*
       * If there's something still running, insert a thunk.
       * This also signals to searchResultTransform.ts to pre-empt the query.
       */
      that._mrlNextAction = () => Promise.resolve();
    } else {
      /*
       * Otherwise, nothing to do.
       *
       * Set the nextAction to undefined.
       * It should never be defined at this point, but just in case.
       */

      that._mrlNextAction = undefined;
    }
    // -----------------------------------
  }

  _mrlQueueNewQuery(queryString: string) {
    this._mrlRefreshEmptyInputClass();
    this._mrlRunOrQueue(() => this._mrlRunNewQuery(queryString));
  }

  private async _mrlRunNewQuery(queryString: string): Promise<void> {
    const that = this;

    const options = that._mrlOptions;
    const { resultsPerPage } = options.uiOptions;

    unsetActiveDescendant(that._mrlInputEl);
    setExpanded(that._mrlInputEl);

    // const now = performance.now();

    // --------------------------------------------------------------
    // Extract enum filters
    const enumFilters = Object.create(null);
    that._mrlFiltersStates.forEach((state) => {
      if (state._mrlIsEnumActive.every((a) => a)) {
        // If all boxes are ticked, don't even add the filter
        return;
      }

      // null is the default, unspecified enum value for documents
      const filters: (string | null)[] = [];
      enumFilters[state._mrlFieldName] = filters;
      if (state._mrlIsEnumActive[0]) filters.push(null);
      filters.push(
        ...state._mrlEnumNames.filter((_, idx) => idx > 0 && state._mrlIsEnumActive[idx]),
      );
    });
    // --------------------------------------------------------------

    if (that._mrlCurrQuery) that._mrlCurrQuery.free();

    const searcher = that._mrlSearcher;
    that._mrlCurrQuery = await searcher.runQuery(queryString, { enumFilters });

    // console.log(`runQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    await loadQueryResults(
      searcher,
      that,
      // Lock-in the query
      that._mrlCurrQuery,
      resultsPerPage,
      0,
      options,
    );

    that._mrlScroller.scrollTo({ top: 0 });
  }

  private async _mrlRunOrQueue(f: () => Promise<any>) {
    const that = this;
    that._mrlRefreshLoader(false);

    if (that._mrlIsRunningAction) {
      that._mrlNextAction = f;
      return;
    }

    that._mrlIsRunningAction = true;
    try {
      await f();
      that._mrlRefreshLoader();
    } catch (ex) {
      that._mrlRefreshLoader(true, true);
      console.error(ex);
    } finally {
      that._mrlIsRunningAction = false;

      // Run the next queued action if there is one
      if (that._mrlNextAction) {
        const nextAction = that._mrlNextAction;
        that._mrlNextAction = undefined;
        await that._mrlRunOrQueue(nextAction);
      }
    }
  }
}
