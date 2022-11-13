import { Searcher } from '@infisearch/search-lib';
import { Options, UiMode } from './Options';
import { IManager } from './InputManager';
import { prepareOptions } from './search/options';
import {
  openDropdown, closeDropdown,
  dropdownRootRender, fsRootRender, setFsTriggerInput,
  setDropdownInputAria, unsetDropdownInputAria, targetRender,
} from './search/rootContainers';

// State / handlers for a single infisearch.init() call
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
    scrollContainer: HTMLElement,
    searcher: Searcher,
    options: Options,
  ) {
    const { uiOptions } = options;
  
    /*
     Behaviour:
     - Wait for the **first** run of the previous active query to finish before running a new one.
     - Do not wait for subsequent runs however -- should be able to "change queries" quickly
     */
    const iManager = new IManager(input, searcher, scrollContainer, options);

    let setupOk = true;
    searcher.setupPromise.catch(() => setupOk = false);

    let inputTimer: any = -1;
    input.addEventListener('input', (ev: InputEvent) => {
      if (!setupOk) {
        return;
      }

      const query = uiOptions.preprocessQuery((ev.target as HTMLInputElement).value);
    
      clearTimeout(inputTimer);
      if (query.length) {
        // Debounce queries
        inputTimer = setTimeout(() => {
          if (this._mrlShowDropdown) this._mrlShowDropdown();
          iManager._mrlQueueNewQuery(query);
        }, uiOptions.inputDebounce);
      } else {
        // But resets should be instant

        iManager._mrlReset();

        if (this._mrlUseDropdown()) {
          // Dropdown, hide it
          this._mrlHideDropdown();
        }
      }
    });
  }
}

function init(options: Options): {
  showFullscreen: () => void,
  hideFullscreen: () => void,
} {
  prepareOptions(options);

  const { uiOptions } = options;
  const {
    input, mode,
    dropdownAlignment,
    label,
    fsInputButtonText, fsInputLabel, fsScrollLock,
    target,
  } = uiOptions;

  const searcher = new Searcher(options.searcherOptions);

  const initState = new InitState(options);


  // --------------------------------------------------
  // Fullscreen version
  const [fsScrollContainer, fsInput, openFullscreen, closeFullscreen] = fsRootRender(
    options,
    (isKeyboardClose) => {
      if (isKeyboardClose && input) input.focus();
      initState._mrlFsShown = false;
      if (fsScrollLock) {
        document.body.style.overflow = '';
      }
    },
  );

  initState._mrlCreateInputListener(
    fsInput, fsScrollContainer, searcher, options,
  );

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

  if (input && (mode === UiMode.Auto || mode === UiMode.Dropdown)) {
    // Auto / Dropdown

    const originalPlaceholder = input.getAttribute('placeholder') || '';

    const parent = input.parentElement;
    const parentChildNodes = parent.childNodes;

    let inputIdx = 0;
    for (; inputIdx < parentChildNodes.length && parentChildNodes[inputIdx] !== input; inputIdx += 1);

    input.remove();
    const [dropdownRoot, dropdownScroller] = dropdownRootRender(options, input, () => {
      initState._mrlHideDropdown();
    });
    
    if (inputIdx < parentChildNodes.length) {
      parent.insertBefore(dropdownRoot, parentChildNodes[inputIdx]);
    } else {
      parent.appendChild(dropdownRoot);
    }

    initState._mrlShowDropdown = () => {
      if (input.value) {
        // Show the dropdown only if it is not empty
        openDropdown(input, dropdownRoot, dropdownScroller, dropdownAlignment);
        initState._mrlDropdownShown = true;
      }
    };
    initState._mrlHideDropdown = () => {
      closeDropdown(dropdownRoot);
      initState._mrlDropdownShown = false;
    };

    initState._mrlCreateInputListener(
      input, dropdownScroller, searcher, options,
    );

    const resultContainer = dropdownScroller.children[3] as HTMLElement;
    function toggleUiMode() {
      if (initState._mrlUseDropdown()) {
        hideFullscreen();
        if (initState._mrlDropdownShown || document.activeElement === input) {
          // If it is already shown, trigger a resize.
          // Otherwise, the input should be focused
          initState._mrlShowDropdown();
        }
        setDropdownInputAria(input, resultContainer, label, originalPlaceholder);
      } else {
        initState._mrlHideDropdown();
        unsetDropdownInputAria(input, resultContainer, fsInputLabel, fsInputButtonText);
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
    targetRender(options, input, target);

    initState._mrlCreateInputListener(input, target, searcher, options);
  }
  // --------------------------------------------------

  return {
    showFullscreen,
    hideFullscreen,
  };
}

export default init;
