import { Searcher } from '@morsels/search-lib';
import loadQueryResults from './searchResultTransform';
import { SearchUiOptions, UiMode, UiOptions } from './SearchUiOptions';
import createElement, { createInvisibleLoadingIndicator, LOADING_INDICATOR_ID } from './utils/dom';
import { InputState } from './utils/input';
import { prepareOptions } from './search/options';
import { setCombobox, setInputAria } from './utils/aria';
import {
  openDropdown, openFullscreen,
  closeDropdown, closeFullscreen,
  dropdownRootRender, fsRootRender,
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
  let indicatorElement: { v: HTMLElement } = { v: createInvisibleLoadingIndicator() };

  /*
   Behaviour:
   - Wait for the **first** run of the previous active query to finish before running a new one.
   - Do not wait for subsequent runs however -- should be able to "change queries" quickly
   */
  const inputState = new InputState();
  async function runNewQuery(queryString: string): Promise<void> {
    const newIndicatorElement = uiOptions.loadingIndicatorRender(
      createElement, options, false, inputState.isResultsBlank,
    );
    indicatorElement.v.replaceWith(newIndicatorElement);
    indicatorElement.v = newIndicatorElement;

    try {
      // const now = performance.now();
  
      inputState.currQuery?.free();
      inputState.currQuery = await searcher.getQuery(queryString);
  
      // console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);
  
      const resultsDisplayed = await loadQueryResults(
        inputState, inputState.currQuery, searcher.config,
        true,
        listContainer, indicatorElement,
        options,
      );
      if (resultsDisplayed) {
        inputState.isResultsBlank = false;
      }
  
      root.scrollTo({ top: 0 });
      listContainer.scrollTo({ top: 0 });
    } catch (ex) {
      console.error(ex);
      listContainer.innerHTML = '';
      listContainer.appendChild(uiOptions.errorRender(createElement, options));
      throw ex;
    } finally {
      if (inputState.nextAction) {
        const nextActionTemp = inputState.nextAction;
        inputState.nextAction = undefined;
        await nextActionTemp();
      } else {
        inputState.isRunningQuery = false;
      }
    }
  }

  searcher.setupPromise.then(() => {
    if (inputState.nextAction) {
      inputState.nextAction();
      inputState.nextAction = undefined;
    }
  });

  let inputTimer: any = -1;
  return (ev: InputEvent) => {
    const query = uiOptions.preprocessQuery((ev.target as HTMLInputElement).value);
  
    clearTimeout(inputTimer);
    if (query.length) {
      inputTimer = setTimeout(() => {
        if (inputState.isResultsBlank
          && !listContainer.firstElementChild?.getAttribute(LOADING_INDICATOR_ID)) {
          listContainer.innerHTML = '';
          indicatorElement.v = uiOptions.loadingIndicatorRender(
            createElement, options, !searcher.isSetupDone, true,
          );
          listContainer.appendChild(indicatorElement.v);
  
          if (useDropdown(uiOptions)) {
            showDropdown();
          }
        }
  
        if (inputState.isRunningQuery || !searcher.isSetupDone) {
          inputState.nextAction = () => runNewQuery(query);
        } else {
          inputState.isRunningQuery = true;
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
  
        inputState.isRunningQuery = false;
        inputState.isResultsBlank = true;
      };
  
      if (inputState.isRunningQuery) {
        inputState.nextAction = reset;
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
  prepareOptions(options, isMobileSizeGlobal);

  const { uiOptions } = options;

  const searcher = new Searcher(options.searcherOptions);


  // --------------------------------------------------
  // Fullscreen version
  let hideFullscreen: () => void;
  const {
    root: fsRoot,
    listContainer: fsListContainer,
    input: fsInput,
  } = fsRootRender(options, () => hideFullscreen());

  fsInput.addEventListener('input', createInputListener(fsRoot, fsListContainer, searcher, options));

  // Initial state is blank
  fsListContainer.appendChild(uiOptions.fsBlankRender(createElement, options));

  const showFullscreen = () => {
    openFullscreen(fsRoot, fsListContainer, uiOptions.fsContainer);
    fullscreenShown = true;
  };
  hideFullscreen = () => {
    closeFullscreen(fsRoot);
    fullscreenShown = false;
  };
  // --------------------------------------------------

  // --------------------------------------------------
  // Input element option handling
  let dropdownListContainer;
  const { input, dropdownAlignment } = uiOptions;
  if (input && (uiOptions.mode === UiMode.Auto || uiOptions.mode === UiMode.Dropdown)) {
    // Auto / Dropdown

    const parent = input.parentElement;
    input.remove();
    const {
      dropdownRoot, dropdownListContainer: dropdownListContainerr,
    } = dropdownRootRender(options, input);
    dropdownListContainer = dropdownListContainerr;
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

    let debounce;
    window.addEventListener('resize', () => {
      clearTimeout(debounce);
      debounce = setTimeout(() => {
        if (uiOptions.mode === UiMode.Dropdown) {
          refreshDropdown();
          return;
        }

        isMobileSizeGlobal = isMobileDevice();
        if (isMobileSizeGlobal) {
          hideDropdown();
        } else {
          hideFullscreen();
          refreshDropdown();
        }
      }, 10);
    });

    input.addEventListener('blur', () => {
      if (useDropdown(uiOptions)) {
        setTimeout(() => {
          let activeEl = document.activeElement;
          while (activeEl) {
            activeEl = activeEl.parentElement;
            if (activeEl === dropdownRoot) {
              input.focus();
              return;
            }
          }
          hideDropdown();
        }, 100);
      }
    });

    input.addEventListener('focus', () => {
      if (useDropdown(uiOptions)) {
        showDropdown();
        return;
      }

      // When using 'auto' mode, may still be using fullscreen UI
      showFullscreen();
    });
  } else if (input && uiOptions.mode === UiMode.Fullscreen) {
    // Fullscreen-only mode
    input.addEventListener('focus', () => {
      showFullscreen();
    });
  } else if (input && uiOptions.mode === UiMode.Target) {
    // Target
    input.addEventListener(
      'input',
      createInputListener(uiOptions.target, uiOptions.target, searcher, options),
    );

    let ariaControlsId = uiOptions.target.getAttribute('id');
    if (!ariaControlsId) {
      uiOptions.target.setAttribute('id', 'morsels-target-list');
      ariaControlsId = 'morsels-target-list';
    }

    setInputAria(input, ariaControlsId);
    setCombobox(input, uiOptions.target, uiOptions.label);
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
    } else if (uiOptions.mode === UiMode.Target) {
      listContainer = uiOptions.target;
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
