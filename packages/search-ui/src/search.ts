import { Searcher } from '@morsels/search-lib';
import { Options, UiMode, UiOptions } from './Options';
import createElement, { LOADING_INDICATOR_ID, MISC_INFO_ID } from './utils/dom';
import { InputState, runNewQuery } from './utils/input';
import { prepareOptions } from './search/options';
import { setCombobox, setInputAria } from './utils/aria';
import {
  openDropdown, closeDropdown,
  dropdownRootRender, fsRootRender, setDropdownInputAria, unsetDropdownInputAria, setFsTriggerInput,
} from './search/rootContainers';

let isMobileSizeGlobal = false;

function useDropdown(uiOptions: UiOptions): boolean {
  return (uiOptions.mode === UiMode.Auto && !isMobileSizeGlobal)
      || uiOptions.mode === UiMode.Dropdown;
}

// State / handlers for a single initMorsels() call
class InitState {
  _mrlShowDropdown: () => void;

  _mrlHideDropdown: () => void;

  _mrlDropdownShown = false;

  _mrlFsShown = false;

  _mrlCreateInputListener(
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
    const inputState = new InputState();
  
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
        listContainer.appendChild(uiOptions.errorRender(createElement, options));
        setupOk = false;
      });
  
    let inputTimer: any = -1;
    return (ev: InputEvent) => {
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

            if (useDropdown(uiOptions)) {
              this._mrlShowDropdown();
            }
          }

          // Queue or immediately run the query
          const action = () => runNewQuery(query, inputState, searcher, root, listContainer, options);
          if (inputState._mrlIsRunningQuery || !searcher.isSetupDone) {
            inputState._mrlNextAction = action;
          } else {
            action();
          }
        }, uiOptions.inputDebounce);
      } else {
        // Resets should be instant

        const reset = () => {
          listContainer.innerHTML = '';

          if (useDropdown(uiOptions)) {
            // Dropdown, hide it
            this._mrlHideDropdown();
          } else if (uiOptions.mode !== UiMode.Target) {
            // Fullscreen, render the initial element / text
            listContainer.appendChild(uiOptions.fsBlankRender(createElement, options));
          } /* else {
            // Target, no action needed other than clearing the HTML
          } */
    
          inputState._mrlIsRunningQuery = false;
          inputState._mrlIsResultsBlank = true;
        };
    
        if (inputState._mrlIsRunningQuery) {
          inputState._mrlNextAction = reset;
        } else {
          reset();
        }
      }
    };
  }
}

const searchers: { [url: string]: Searcher } = {};

function initMorsels(options: Options): {
  showFullscreen: () => void,
  hideFullscreen: () => void,
} {
  const isMobileDevice: () => boolean = options.isMobileDevice
      || (() => window.matchMedia('only screen and (max-width: 768px)').matches);

  isMobileSizeGlobal = isMobileDevice();
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

  const initState = new InitState();


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

  fsInput.addEventListener(
    'input',
    initState._mrlCreateInputListener(fsRoot, fsListContainer, searcher, options),
  );

  // Initial state is blank
  fsListContainer.appendChild(uiOptions.fsBlankRender(createElement, options));

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
      if (!useDropdown(uiOptions)) {
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
      openDropdown(dropdownRoot, dropdownListContainer, dropdownAlignment);
      initState._mrlDropdownShown = true;
    };
    initState._mrlHideDropdown = () => {
      closeDropdown(dropdownRoot);
      initState._mrlDropdownShown = false;
    };

    input.addEventListener(
      'input',
      initState._mrlCreateInputListener(dropdownRoot, dropdownListContainer, searcher, options),
    );

    function refreshDropdown() {
      initState._mrlHideDropdown();
      if (document.activeElement === input) {
        initState._mrlShowDropdown();
      }
    }

    function toggleUiMode() {
      if ((mode === UiMode.Dropdown)
        || !(isMobileSizeGlobal = isMobileDevice())) {
        hideFullscreen();
        refreshDropdown();
        setDropdownInputAria(input, dropdownRoot, dropdownListContainer, label, originalPlaceholder);
      } else {
        initState._mrlHideDropdown();
        unsetDropdownInputAria(dropdownRoot, dropdownListContainer, input, fsInputLabel, fsInputButtonText);
      }
    }
    toggleUiMode();

    let resizeDebounce;
    window.addEventListener('resize', () => {
      clearTimeout(resizeDebounce);
      resizeDebounce = setTimeout(toggleUiMode, 10);
    });

    dropdownRoot.addEventListener('focusout', () => {
      if (useDropdown(uiOptions)) {
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

    input.addEventListener('focus', () => useDropdown(uiOptions) && initState._mrlShowDropdown());
    addFsTriggerInputListeners();
  } else if (input && mode === UiMode.Fullscreen) {
    // Fullscreen-only mode
    setFsTriggerInput(input, fsInputButtonText, fsInputLabel);
    addFsTriggerInputListeners();
  } else if (input && mode === UiMode.Target) {
    // Target

    target.classList.add('morsels-root');

    input.addEventListener(
      'input',
      initState._mrlCreateInputListener(target, target, searcher, options),
    );

    let ariaControlsId = target.getAttribute('id');
    if (!ariaControlsId) {
      target.setAttribute('id', 'morsels-target-list');
      ariaControlsId = 'morsels-target-list';
    }

    setInputAria(input, ariaControlsId);
    setCombobox(input, target, uiOptions.label);
  }
  // --------------------------------------------------

  // --------------------------------------------------
  // Keyboard Events

  function keydownListener(ev: KeyboardEvent) {
    const { key } = ev;
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter'].includes(key)) {
      return;
    }

    let listContainer: HTMLElement;

    let scrollListContainer = (targetEl: any) => {
      const top = targetEl.offsetTop
        - listContainer.offsetTop
        - listContainer.clientHeight / 2
        + targetEl.clientHeight / 2;
      listContainer.scrollTo({ top });
    };

    const isDropdown = useDropdown(uiOptions);
    if (isDropdown) {
      if (!initState._mrlDropdownShown) {
        return;
      }

      listContainer = dropdownListContainer;
    } else if (mode === UiMode.Target) {
      listContainer = target;
      scrollListContainer = (targetEl: HTMLElement) => {
        targetEl.scrollIntoView({
          block: 'center',
        });
      };
    } else {
      if (!initState._mrlFsShown) {
        return;
      }

      listContainer = fsListContainer;
    }

    const focusedItem = listContainer.querySelector('.focus');
    function focusEl(el: Element) {
      if (el && !el.getAttribute(MISC_INFO_ID) && !el.getAttribute(LOADING_INDICATOR_ID)) {
        if (focusedItem) {
          focusedItem.classList.remove('focus');
          focusedItem.removeAttribute('aria-selected');
          focusedItem.removeAttribute('id');
        }

        el.classList.add('focus');
        el.setAttribute('aria-selected', 'true');
        el.setAttribute('id', 'morsels-list-selected');
        scrollListContainer(el);

        return true;
      }

      return false;
    }

    const firstItem = listContainer.querySelector(`[${MISC_INFO_ID}]`)?.nextElementSibling;
    const lastItem = listContainer.lastElementChild;
    if (key === 'ArrowDown') {
      if (focusedItem) {
        focusEl(focusedItem.nextElementSibling);
      } else {
        focusEl(firstItem);
      }
    } else if (key === 'ArrowUp') {
      if (focusedItem) {
        focusEl(focusedItem.previousElementSibling);
      }
    } else if (key === 'Home') {
      focusEl(firstItem);
    } else if (key === 'End') {
      if (!focusEl(lastItem)) {
        focusEl(lastItem?.previousElementSibling);
      }
    } else if (key === 'Enter') {
      if (focusedItem) {
        const link = focusedItem.querySelector('a[href]');
        if (link) {
          window.location.href = link.getAttribute('href');
        }
      }
    }

    ev.preventDefault();
  }

  input?.addEventListener('keydown', keydownListener);
  fsInput.addEventListener('keydown', keydownListener);
  
  // --------------------------------------------------

  return {
    showFullscreen,
    hideFullscreen,
  };
}

export default initMorsels;
