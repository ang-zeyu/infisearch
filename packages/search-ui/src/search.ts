import { Searcher } from '@morsels/search-lib';
import createElement from '@morsels/search-lib/lib/utils/dom';
import { Options, UiMode } from './Options';
import { LOADING_INDICATOR_ID } from './utils/dom';
import { InputState, runNewQuery } from './utils/input';
import { prepareOptions } from './search/options';
import {
  unsetActiveDescendant,
  setExpanded,
  setInputAria,
  unsetExpanded,
} from './utils/aria';
import {
  openDropdown, closeDropdown,
  dropdownRootRender, fsRootRender, setFsTriggerInput,
  setDropdownInputAria, unsetDropdownInputAria,
} from './search/rootContainers';

// State / handlers for a single morsels.initMorsels() call
class InitState {
  _mrlShowDropdown: () => void;

  _mrlHideDropdown: () => void;

  _mrlDropdownShown = false;

  _mrlFsShown = false;

  constructor(private _mrlOpts: Options) {}

  _mrlUseDropdown() {
    const { mode, isMobileDevice } = this._mrlOpts.uiOptions;
    return  (mode === UiMode.Auto && !isMobileDevice())
      || mode === UiMode.Dropdown;
  }

  _mrlCreateInputListener(
    input: HTMLInputElement,
    root: HTMLElement,
    listContainer: HTMLElement,
    searcher: Searcher,
    options: Options,
  ) {
    const { uiOptions } = options;
  
    /*
     Behaviour:
     - Wait for the **first** run of the previous active query to finish before running a new one.
     - Do not wait for subsequent runs however -- should be able to "change queries" quickly
     */
    const inputState = new InputState(input, listContainer);
  
    let setupOk = true;
    searcher.setupPromise
      .then(() => {
        if (inputState._mrlNextAction) {
          inputState._mrlNextAction();
          inputState._mrlNextAction = undefined;
        }
      })
      .catch(() => {
        listContainer.innerHTML = '';
        listContainer.appendChild(uiOptions.headerRender(createElement, options, true, false));
        setupOk = false;
      });
  
    let inputTimer: any = -1;
    input.addEventListener('input', (ev: InputEvent) => {
      if (!setupOk) {
        return;
      }

      const query = uiOptions.preprocessQuery((ev.target as HTMLInputElement).value);
    
      clearTimeout(inputTimer);
      if (query.length) {
        // Only debounce queries

        inputTimer = setTimeout(() => {
          if (
            inputState._mrlIsResultsBlank
            && !listContainer.firstElementChild?.getAttribute(LOADING_INDICATOR_ID)
          ) {
            /*
             The first ever query for this input.
             Add the setup loading indicator (if not done)
             or the normal query loading indicator.
            */

            listContainer.innerHTML = '';

            const loader = uiOptions.loadingIndicatorRender(
              createElement, options, !searcher.isSetupDone, true,
            );
            inputState._mrlLoader = loader;
            listContainer.appendChild(loader);
          }

          if (this._mrlShowDropdown) this._mrlShowDropdown();

          // Queue or immediately run the query
          const action = () => runNewQuery(query, inputState, searcher, root, listContainer, options);
          if (inputState._mrlIsRunningQuery || !searcher.isSetupDone) {
            inputState._mrlNextAction = action;
          } else {
            action();
          }

          unsetActiveDescendant(input);
          setExpanded(input);
        }, uiOptions.inputDebounce);
      } else {
        // Resets should be instant

        const reset = () => {
          listContainer.innerHTML = '';

          if (this._mrlUseDropdown()) {
            // Dropdown, hide it
            this._mrlHideDropdown();
          } else if (uiOptions.mode !== UiMode.Target) {
            // Fullscreen, render the initial element / text
            listContainer.appendChild(uiOptions.headerRender(createElement, options, false, true));
          } /* else {
            // Target, no action needed other than clearing the HTML
          } */
    
          inputState._mrlIsRunningQuery = false;
          inputState._mrlIsResultsBlank = true;
          unsetActiveDescendant(input);
          unsetExpanded(input);
        };
    
        if (inputState._mrlIsRunningQuery) {
          inputState._mrlNextAction = reset;
        } else {
          reset();
        }
      }
    });
  }
}

const searchers: { [url: string]: Searcher } = {};

function initMorsels(options: Options): {
  showFullscreen: () => void,
  hideFullscreen: () => void,
} {
  prepareOptions(options);

  const { uiOptions, searcherOptions } = options;
  const {
    input, mode,
    dropdownAlignment,
    label,
    fsInputButtonText, fsInputLabel, fsScrollLock,
    target,
  } = uiOptions;
  const { url } = searcherOptions;

  if (!searchers[url]) {
    searchers[url] = new Searcher(options.searcherOptions);
  }
  const searcher = searchers[url];

  const initState = new InitState(options);


  // --------------------------------------------------
  // Fullscreen version
  const [fsRoot, fsListContainer, fsInput, openFullscreen, closeFullscreen] = fsRootRender(
    options, searcher,
    (isKeyboardClose) => {
      if (isKeyboardClose && input) input.focus();
      initState._mrlFsShown = false;
      if (fsScrollLock) {
        document.body.style.overflow = '';
      }
    },
  );

  initState._mrlCreateInputListener(
    fsInput, fsRoot, fsListContainer, searcher, options,
  );

  // Initial state is blank
  fsListContainer.appendChild(uiOptions.headerRender(createElement, options, false, true));

  function showFullscreen() {
    if (!initState._mrlFsShown) {
      openFullscreen();
      initState._mrlFsShown = true;
      if (fsScrollLock) {
        document.body.style.overflow = 'hidden';
      }
    }
  }

  function hideFullscreen() {
    closeFullscreen(false);
    if (fsScrollLock) {
      document.body.style.overflow = '';
    }
  }

  function addFsTriggerInputListeners() {
    function showFsIfNotDropdown() {
      if (!initState._mrlUseDropdown()) {
        showFullscreen();
      }
    }

    input.addEventListener('click', showFsIfNotDropdown);
    input.addEventListener('keydown', (ev: KeyboardEvent) => {
      if (ev.key === 'Enter') {
        showFsIfNotDropdown();
      }
    });
  }
  // --------------------------------------------------

  // --------------------------------------------------
  // Input element option handling
  let dropdownListContainer: HTMLElement;
  if (input && (mode === UiMode.Auto || mode === UiMode.Dropdown)) {
    // Auto / Dropdown

    const originalPlaceholder = input.getAttribute('placeholder') || '';

    const parent = input.parentElement;
    const parentChildNodes = parent.childNodes;

    let inputIdx = 0;
    for (; inputIdx < parentChildNodes.length && parentChildNodes[inputIdx] !== input; inputIdx += 1);

    input.remove();
    const [dropdownRoot, d] = dropdownRootRender(uiOptions, searcher, input, () => {
      initState._mrlHideDropdown();
    });
    dropdownListContainer = d;
    if (inputIdx < parentChildNodes.length) {
      parent.insertBefore(dropdownRoot, parentChildNodes[inputIdx]);
    } else {
      parent.appendChild(dropdownRoot);
    }

    initState._mrlShowDropdown = () => {
      if (dropdownListContainer.childElementCount) {
        openDropdown(dropdownRoot, dropdownListContainer, dropdownAlignment);
        initState._mrlDropdownShown = true;
      }
    };
    initState._mrlHideDropdown = () => {
      closeDropdown(dropdownRoot);
      initState._mrlDropdownShown = false;
    };

    initState._mrlCreateInputListener(
      input, dropdownRoot, dropdownListContainer, searcher, options,
    );

    function toggleUiMode() {
      if (initState._mrlUseDropdown()) {
        hideFullscreen();
        if (initState._mrlDropdownShown || document.activeElement === input) {
          // If it is already shown, trigger a resize.
          // Otherwise, the input should be focused
          initState._mrlShowDropdown();
        }
        setDropdownInputAria(input, dropdownListContainer, label, originalPlaceholder);
      } else {
        initState._mrlHideDropdown();
        unsetDropdownInputAria(input, dropdownListContainer, fsInputLabel, fsInputButtonText);
      }
    }
    toggleUiMode();

    // Note: on mobile, keyboard show/hides also trigger this
    let resizeDebounce;
    window.addEventListener('resize', () => {
      clearTimeout(resizeDebounce);
      resizeDebounce = setTimeout(toggleUiMode, 10);
    });

    dropdownRoot.addEventListener('focusout', () => {
      if (initState._mrlUseDropdown()) {
        setTimeout(() => {
          let activeEl = document.activeElement;
          while (activeEl) {
            activeEl = activeEl.parentElement;
            if (activeEl === dropdownRoot) {
              return;
            }
          }
          initState._mrlHideDropdown();
        }, 100);
      }
    });

    input.addEventListener('focus', () => {
      if (!initState._mrlDropdownShown && initState._mrlUseDropdown()) {
        initState._mrlShowDropdown();
      }
    });
    addFsTriggerInputListeners();
  } else if (input && mode === UiMode.Fullscreen) {
    // Fullscreen-only mode
    setFsTriggerInput(input, fsInputButtonText, fsInputLabel);
    addFsTriggerInputListeners();
  } else if (input && mode === UiMode.Target) {
    // Target

    target.classList.add('morsels-root');

    initState._mrlCreateInputListener(input, target, target, searcher, options);

    let ariaControlsId = target.getAttribute('id');
    if (!ariaControlsId) {
      target.setAttribute('id', 'morsels-target-list');
      ariaControlsId = 'morsels-target-list';
    }

    setInputAria(input, target, uiOptions.label);
  }
  // --------------------------------------------------

  return {
    showFullscreen,
    hideFullscreen,
  };
}

export default initMorsels;
