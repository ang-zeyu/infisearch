import { Searcher } from '@morsels/search-lib';
import loadQueryResults from './searchResultTransform';
import { SearchUiOptions, UiMode, UiOptions } from './SearchUiOptions';
import createElement, { LOADING_INDICATOR_ID } from './utils/dom';
import { InputState } from './utils/input';
import { prepareOptions } from './search/options';
import { setCombobox, setInputAria } from './utils/aria';
import {
  openDropdown, closeDropdown,
  dropdownRootRender, fsRootRender, setDropdownInputAria, unsetDropdownInputAria, setFsTriggerInput,
} from './search/rootContainers';

let isMobileSizeGlobal = false;

let showDropdown: () => void;
let hideDropdown: () => void;
let dropdownShown = false;
let fullscreenShown = false;


function useDropdown(uiOptions: UiOptions): boolean {
  return (uiOptions.mode === UiMode.Auto && !isMobileSizeGlobal)
      || uiOptions.mode === UiMode.Dropdown;
}

function createInputListener(
  root: HTMLElement,
  listContainer: HTMLElement,
  searcher: Searcher,
  options: SearchUiOptions,
) {
  const { uiOptions } = options;

  /*
   Behaviour:
   - Wait for the **first** run of the previous active query to finish before running a new one.
   - Do not wait for subsequent runs however -- should be able to "change queries" quickly
   */
  const inputState = new InputState();
  async function runNewQuery(queryString: string): Promise<void> {
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
      console.error(ex);
      listContainer.innerHTML = '';
      listContainer.appendChild(uiOptions.errorRender(createElement, options));
      throw ex;
    } finally {
      if (inputState._mrlNextAction) {
        const nextActionTemp = inputState._mrlNextAction;
        inputState._mrlNextAction = undefined;
        await nextActionTemp();
      } else {
        inputState._mrlIsRunningQuery = false;
      }
    }
  }

  searcher.setupPromise.then(() => {
    if (inputState._mrlNextAction) {
      inputState._mrlNextAction();
      inputState._mrlNextAction = undefined;
    }
  });

  let inputTimer: any = -1;
  return (ev: InputEvent) => {
    const query = uiOptions.preprocessQuery((ev.target as HTMLInputElement).value);
  
    clearTimeout(inputTimer);
    if (query.length) {
      inputTimer = setTimeout(() => {
        if (inputState._mrlIsResultsBlank
          && !listContainer.firstElementChild?.getAttribute(LOADING_INDICATOR_ID)) {
          listContainer.innerHTML = '';
          inputState._mrlLoader = uiOptions.loadingIndicatorRender(
            createElement, options, !searcher.isSetupDone, true,
          );
          listContainer.appendChild(inputState._mrlLoader);
  
          if (useDropdown(uiOptions)) {
            showDropdown();
          }
        }
  
        if (inputState._mrlIsRunningQuery || !searcher.isSetupDone) {
          inputState._mrlNextAction = () => runNewQuery(query);
        } else {
          runNewQuery(query);
        }
      }, uiOptions.inputDebounce);
    } else {
      const reset = () => {
        listContainer.innerHTML = '';
        if (uiOptions.mode !== UiMode.Target) {
          if (useDropdown(uiOptions)) {
            hideDropdown();
          } else {
            // useFullscreen
            listContainer.appendChild(uiOptions.fsBlankRender(createElement, options));
          }
        }
  
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


function initMorsels(options: SearchUiOptions): {
  showFullscreen: () => void,
  hideFullscreen: () => void,
} {
  const isMobileDevice: () => boolean = options.isMobileDevice
      || (() => window.matchMedia('only screen and (max-width: 1024px)').matches);

  isMobileSizeGlobal = isMobileDevice();
  prepareOptions(options);

  const { uiOptions } = options;
  const {
    input, mode,
    dropdownAlignment,
    label, fsInputButtonText, fsInputLabel,
    target,
  } = uiOptions;

  const searcher = new Searcher(options.searcherOptions);


  // --------------------------------------------------
  // Fullscreen version
  const [fsRoot, fsListContainer, fsInput, openFullscreen, closeFullscreen] = fsRootRender(
    options, searcher,
    (isKeyboardClose) => {
      if (isKeyboardClose && input) input.focus();
      fullscreenShown = false;
    },
  );

  fsInput.addEventListener('input', createInputListener(fsRoot, fsListContainer, searcher, options));

  // Initial state is blank
  fsListContainer.appendChild(uiOptions.fsBlankRender(createElement, options));

  function showFullscreen() {
    if (!fullscreenShown) {
      openFullscreen();
      fullscreenShown = true;
    }
  }

  function hideFullscreen() {
    closeFullscreen(false);
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
    input.remove();
    const [dropdownRoot, d] = dropdownRootRender(uiOptions, searcher, input);
    dropdownListContainer = d;
    parent.appendChild(dropdownRoot);

    showDropdown = () => {
      openDropdown(dropdownRoot, dropdownListContainer, dropdownAlignment);
      dropdownShown = true;
    };
    hideDropdown = () => {
      closeDropdown(dropdownRoot);
      dropdownShown = false;
    };

    input.addEventListener(
      'input',
      createInputListener(dropdownRoot, dropdownListContainer, searcher, options),
    );

    function refreshDropdown() {
      hideDropdown();
      if (document.activeElement === input) {
        showDropdown();
      }
    }

    function toggleUiMode() {
      if ((mode === UiMode.Dropdown)
        || !(isMobileSizeGlobal = isMobileDevice())) {
        hideFullscreen();
        refreshDropdown();
        setDropdownInputAria(input, dropdownRoot, dropdownListContainer, label, originalPlaceholder);
      } else {
        hideDropdown();
        unsetDropdownInputAria(dropdownRoot, dropdownListContainer, input, fsInputLabel, fsInputButtonText);
      }
    }
    toggleUiMode();

    let debounce;
    window.addEventListener('resize', () => {
      clearTimeout(debounce);
      debounce = setTimeout(toggleUiMode, 10);
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
          hideDropdown();
        }, 100);
      }
    });

    input.addEventListener('focus', () => useDropdown(uiOptions) && showDropdown());
    addFsTriggerInputListeners();
  } else if (input && mode === UiMode.Fullscreen) {
    // Fullscreen-only mode
    setFsTriggerInput(input, fsInputButtonText, fsInputLabel);
    addFsTriggerInputListeners();
  } else if (input && mode === UiMode.Target) {
    // Target
    input.addEventListener(
      'input',
      createInputListener(target, target, searcher, options),
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
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter'].includes(ev.key)) {
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
      if (!dropdownShown) {
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
      if (!fullscreenShown) {
        return;
      }

      listContainer = fsListContainer;
    }

    const focusedItem = listContainer.querySelector('.focus');
    function focusEl(el: Element) {
      if (focusedItem) {
        focusedItem.classList.remove('focus');
        focusedItem.removeAttribute('aria-selected');
        focusedItem.removeAttribute('id');
      }
      el.classList.add('focus');
      el.setAttribute('aria-selected', 'true');
      el.setAttribute('id', 'morsels-list-selected');
      scrollListContainer(el);
    }

    function focusOr(newItem: Element, newItem2: Element) {
      if (newItem && !newItem.getAttribute(LOADING_INDICATOR_ID)) {
        focusEl(newItem);
      } else if (newItem2 && !newItem2.getAttribute(LOADING_INDICATOR_ID)) {
        focusEl(newItem2);
      }
    }

    const firstItem = listContainer.firstElementChild;
    const lastItem = listContainer.lastElementChild;
    if (ev.key === 'ArrowDown') {
      if (focusedItem) {
        focusOr(focusedItem.nextElementSibling, null);
      } else {
        focusOr(firstItem, firstItem?.nextElementSibling);
      }
    } else if (ev.key === 'ArrowUp') {
      if (focusedItem) {
        focusOr(focusedItem.previousElementSibling, null);
      }
    } else if (ev.key === 'Home') {
      focusOr(firstItem, firstItem?.nextElementSibling);
    } else if (ev.key === 'End') {
      focusOr(lastItem, lastItem?.previousElementSibling);
    } else if (ev.key === 'Enter') {
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
