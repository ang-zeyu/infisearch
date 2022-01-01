import { computePosition, size, offset, flip } from '@floating-ui/dom';

import './styles/search.css';

import { Searcher } from '@morsels/search-lib';
import transformResults, { resultsRender } from './searchResultTransform';
import { SearchUiOptions, UiMode, UiOptions } from './SearchUiOptions';
import createElement from './utils/dom';
import { InputState } from './utils/input';
import { parseURL } from './utils/url';

let isMobileSizeGlobal = false;

let dropdownShown = false;
let fullscreenShown = false;


function useDropdown(uiOptions: UiOptions): boolean {
  return (uiOptions.mode === UiMode.Auto && !isMobileSizeGlobal)
      || uiOptions.mode === UiMode.Dropdown;
}

function setCombobox(combobox: HTMLElement, listbox: HTMLElement, label: string) {
  combobox.setAttribute('role', 'combobox');
  combobox.setAttribute('aria-expanded', 'true');
  combobox.setAttribute('aria-owns', listbox.getAttribute('id'));
  listbox.setAttribute('role', 'listbox');
  listbox.setAttribute('aria-label', label);
  listbox.setAttribute('aria-live', 'polite');
}

function setInputAria(input: HTMLElement, listId: string) {
  input.setAttribute('aria-autocomplete', 'list');
  input.setAttribute('aria-controls', listId);
  input.setAttribute('aria-activedescendant', 'morsels-list-selected');
}

function prepareOptions(options: SearchUiOptions) {
  // ------------------------------------------------------------
  // Search Lib Options

  options.searcherOptions = options.searcherOptions || ({} as any);

  if (!('numberOfExpandedTerms' in options.searcherOptions)) {
    options.searcherOptions.numberOfExpandedTerms = 3;
  }

  if (!('useQueryTermProximity' in options.searcherOptions)) {
    options.searcherOptions.useQueryTermProximity = !isMobileSizeGlobal;
  }

  if (!('useWand' in options.searcherOptions)) {
    options.searcherOptions.useWand = 20;
  }

  if (!('resultLimit' in options.searcherOptions)) {
    options.searcherOptions.resultLimit = null; // unlimited
  }

  // ------------------------------------------------------------
  // Ui Options

  options.uiOptions = options.uiOptions || ({} as any);
  const { uiOptions } = options;

  uiOptions.mode = uiOptions.mode || UiMode.Auto;

  if (uiOptions.mode === UiMode.Target) {
    if (typeof uiOptions.target === 'string') {
      uiOptions.target = document.getElementById(uiOptions.target);
    }

    if (!uiOptions.target) {
      throw new Error('\'target\' mode specified but no valid target option specified');
    }
  }

  if (!('input' in uiOptions) || typeof uiOptions.input === 'string') {
    uiOptions.input = document.getElementById(uiOptions.input as any || 'morsels-search') as HTMLInputElement;
  }

  if ([UiMode.Dropdown, UiMode.Target].includes(uiOptions.mode) && !uiOptions.input) {
    throw new Error('\'dropdown\' or \'target\' mode specified but no input element found');
  }

  if (!('inputDebounce' in uiOptions)) {
    uiOptions.inputDebounce = 100;
  }

  if (!uiOptions.preprocessQuery) {
    uiOptions.preprocessQuery = (q) => q;
  }

  if (typeof uiOptions.fsContainer === 'string') {
    uiOptions.fsContainer = document.getElementById(uiOptions.fsContainer) as HTMLElement;
  }

  if (!('dropdownAlignment' in uiOptions)) {
    uiOptions.dropdownAlignment = 'bottom-end';
  }

  if (!uiOptions.fsContainer) {
    uiOptions.fsContainer = document.getElementsByTagName('body')[0] as HTMLElement;
  }

  if (!('resultsPerPage' in uiOptions)) {
    uiOptions.resultsPerPage = 8;
  }

  const showDropdown = uiOptions.showDropdown || ((root, listContainer) => {
    if (listContainer.childElementCount) {
      listContainer.style.display = 'block';
      (listContainer.previousSibling as HTMLElement).style.display = 'block';
      computePosition(root, listContainer, {
        placement: uiOptions.dropdownAlignment,
        middleware: [
          offset(8),
          flip({
            padding: 10,
            mainAxis: false,
          }),
          size({
            apply({ width, height }) {
              Object.assign(listContainer.style, {
                maxWidth: `min(${width}px, var(--morsels-dropdown-max-width))`,
                maxHeight: `min(${height}px, var(--morsels-dropdown-max-height))`,
              });
            },
            padding: 10,
          }),
        ],
      }).then(({ x, y }) => {
        Object.assign(listContainer.style, {
          left: `${x}px`,
          top: `${y}px`,
        });
      });
    }
  });
  uiOptions.showDropdown = (...args) => {
    showDropdown(...args);
    dropdownShown = true;
  };

  const hideDropdown = uiOptions.hideDropdown || ((root, listContainer) => {
    listContainer.style.display = 'none';
    (listContainer.previousSibling as HTMLElement).style.display = 'none';
  });
  uiOptions.hideDropdown = (...args) => {
    hideDropdown(...args);
    dropdownShown = false;
  };

  const showFullscreen = uiOptions.showFullscreen || ((root, listContainer, fsContainer) => {
    fsContainer.appendChild(root);
    const input: HTMLInputElement = root.querySelector('input.morsels-fs-input');
    if (input) {
      input.focus();
    }
  });
  uiOptions.showFullscreen = (root, listContainer, ...args) => {
    showFullscreen(root, listContainer, ...args);
    const currentFocusedResult = listContainer.querySelector('.focus') as HTMLElement;
    if (currentFocusedResult) {
      listContainer.scrollTo({ top: currentFocusedResult.offsetTop - listContainer.offsetTop - 30 });
    }
    fullscreenShown = true;

  };

  const hideFullscreen = uiOptions.hideFullscreen || ((root) => {
    // useFullscreen
    root.remove();
  });
  uiOptions.hideFullscreen = (...args) => {
    hideFullscreen(...args);
    fullscreenShown = false;
  };

  uiOptions.label = uiOptions.label || 'Search this site';
  uiOptions.fsPlaceholder = uiOptions.fsPlaceholder || 'Search this site...';
  uiOptions.fsCloseText = uiOptions.fsCloseText || 'Close';

  uiOptions.dropdownRootRender = uiOptions.dropdownRootRender || ((h, opts, inputEl) => {
    const root = h('div', { class: 'morsels-root' }, inputEl);

    root.appendChild(h('div', {
      class: `morsels-input-dropdown-separator ${uiOptions.dropdownAlignment}`,
      style: 'display: none;',
    }));

    setInputAria(inputEl, 'morsels-dropdown-list');

    const listContainer = h('ul', {
      id: 'morsels-dropdown-list',
      class: 'morsels-list',
      style: 'display: none;',
    });
    root.appendChild(listContainer);
    setCombobox(root, listContainer, opts.uiOptions.label);

    return {
      dropdownRoot: root,
      dropdownListContainer: listContainer,
    };
  });

  uiOptions.fsRootRender = uiOptions.fsRootRender || ((
    h,
    opts,
    fsCloseHandler,
  ) => {
    const innerRoot = h('div', { class: 'morsels-root morsels-fs-root' });
    innerRoot.onclick = (ev) => ev.stopPropagation();

    const rootBackdropEl = h('div', { class: 'morsels-fs-backdrop' }, innerRoot);

    const ariaLabel = h('label',
      { id: 'morsels-fs-label', for: 'morsels-fs-input', style: 'display: none' },
      opts.uiOptions.label,
    );

    const inputEl = h(
      'input', {
        class: 'morsels-fs-input',
        type: 'search',
        placeholder: opts.uiOptions.fsPlaceholder,
        autocomplete: 'false',
        'aria-labelledby': 'morsels-fs-label',
      },
    ) as HTMLInputElement;
    setInputAria(inputEl, 'morsels-fs-list');

    const buttonEl = h('button', { class: 'morsels-input-close-fs' }, opts.uiOptions.fsCloseText);
    buttonEl.onclick = fsCloseHandler;

    innerRoot.appendChild(h('form',
      { class: 'morsels-fs-input-button-wrapper' },
      ariaLabel,
      inputEl,
      buttonEl));

    const listContainer = h('ul', {
      id: 'morsels-fs-list',
      class: 'morsels-list',
      'aria-labelledby': 'morsels-fs-label',
    });
    innerRoot.appendChild(listContainer);
    setCombobox(innerRoot, listContainer, opts.uiOptions.label);

    rootBackdropEl.onclick = () => uiOptions.hideFullscreen(rootBackdropEl, listContainer, innerRoot, opts);
    rootBackdropEl.addEventListener('keyup', (ev) => {
      if (ev.code === 'Escape') {
        ev.stopPropagation();
        uiOptions.hideFullscreen(rootBackdropEl, listContainer, innerRoot, opts);
      }
    });

    return {
      root: rootBackdropEl,
      listContainer,
      input: inputEl,
    };
  });

  uiOptions.errorRender = uiOptions.errorRender
      || ((h) => h('div', { class: 'morsels-error' }, 'Oops! Something went wrong... 🙁'));

  uiOptions.noResultsRender = uiOptions.noResultsRender
      || ((h) => h('div', { class: 'morsels-no-results' }, 'No results found'));

  uiOptions.fsBlankRender = uiOptions.fsBlankRender
      || ((h) => h('div', { class: 'morsels-fs-blank' }, 'Start Searching Above!'));

  uiOptions.loadingIndicatorRender = uiOptions.loadingIndicatorRender
      || ((h) => h('span', { class: 'morsels-loading-indicator' }));

  uiOptions.termInfoRender = uiOptions.termInfoRender || (() => []);

  uiOptions.resultsRender = uiOptions.resultsRender || resultsRender;

  uiOptions.resultsRenderOpts = uiOptions.resultsRenderOpts || {};

  uiOptions.resultsRenderOpts.listItemRender = uiOptions.resultsRenderOpts.listItemRender || ((
    h, opts, searchedTermsJSON, fullLink, title, resultHeadingsAndTexts,
  ) => {
    const linkEl = h(
      'a', { class: 'morsels-link' },
      h('div', { class: 'morsels-title' }, title),
      ...resultHeadingsAndTexts,
    );

    if (fullLink) {
      let linkToAttach = fullLink;
      if (opts.uiOptions.resultsRenderOpts.addSearchedTerms) {
        const fullLinkUrl = parseURL(fullLink);
        fullLinkUrl.searchParams.append(
          options.uiOptions.resultsRenderOpts.addSearchedTerms,
          searchedTermsJSON,
        );
        linkToAttach = fullLinkUrl.toString();
      }
      linkEl.setAttribute('href', linkToAttach);
    }

    return h(
      'li', { class: 'morsels-list-item', role: 'option' },
      linkEl,
    );
  });

  uiOptions.resultsRenderOpts.headingBodyRender = uiOptions.resultsRenderOpts.headingBodyRender
  || ((
    h, opts, heading, bodyHighlights, href,
  ) => {
    const el = h('a', { class: 'morsels-heading-body' },
      h('div', { class: 'morsels-heading' }, heading),
      h('div', { class: 'morsels-bodies' },
        h('div', { class: 'morsels-body' }, ...bodyHighlights)));
    if (href) {
      el.setAttribute('href', href);
    }
    return el;
  });

  uiOptions.resultsRenderOpts.bodyOnlyRender = uiOptions.resultsRenderOpts.bodyOnlyRender || ((
    h, opts, bodyHighlights,
  ) => h(
    'div', { class: 'morsels-body' }, ...bodyHighlights,
  ));

  uiOptions.resultsRenderOpts.highlightRender = uiOptions.resultsRenderOpts.highlightRender || ((
    h, opts, matchedPart,
  ) => h(
    'span', { class: 'morsels-highlight' }, matchedPart,
  ));

  options.otherOptions = options.otherOptions || {};
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
    try {
      // const now = performance.now();
  
      inputState.currQuery?.free();
      inputState.currQuery = await searcher.getQuery(queryString);
  
      // console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);
  
      await transformResults(
        inputState, inputState.currQuery, searcher.morselsConfig, true, listContainer, options,
      );
  
      root.scrollTo({ top: 0 });
      listContainer.scrollTo({ top: 0 });
    } catch (ex) {
      console.error(ex);
      listContainer.innerHTML = '';
      listContainer.appendChild(uiOptions.errorRender(createElement, options));
      throw ex;
    } finally {
      if (inputState.nextQuery) {
        const nextQueryTemp = inputState.nextQuery;
        inputState.nextQuery = undefined;
        await nextQueryTemp();
      } else {
        inputState.isRunningNewQuery = false;
      }
    }
  }

  let inputTimer: any = -1;
  let isFirstQueryFromBlank = true;
  return (ev: InputEvent) => {
    const query = uiOptions.preprocessQuery((ev.target as HTMLInputElement).value);
  
    clearTimeout(inputTimer);
    if (query.length) {
      inputTimer = setTimeout(() => {
        if (isFirstQueryFromBlank) {
          listContainer.innerHTML = '';
          listContainer.appendChild(
            uiOptions.loadingIndicatorRender(createElement, options),
          );
  
          if (useDropdown(uiOptions)) {
            uiOptions.showDropdown(root, listContainer, options);
          }
        }
  
        if (inputState.isRunningNewQuery) {
          inputState.nextQuery = () => runNewQuery(query);
        } else {
          inputState.isRunningNewQuery = true;
          runNewQuery(query);
        }
  
        isFirstQueryFromBlank = false;
      }, uiOptions.inputDebounce);
    } else {
      const reset = () => {
        if (uiOptions.mode === UiMode.Target) {
          listContainer.innerHTML = '';
        } else if (useDropdown(uiOptions)) {
          uiOptions.hideDropdown(root, listContainer, options);
        } else {
          // useFullscreen
          listContainer.innerHTML = '';
          listContainer.appendChild(uiOptions.fsBlankRender(createElement, options));
        }
  
        inputState.isRunningNewQuery = false;
        isFirstQueryFromBlank = true;
      };
  
      if (inputState.isRunningNewQuery) {
        inputState.nextQuery = reset;
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

  const searcher = new Searcher(options.searcherOptions);


  // --------------------------------------------------
  // Fullscreen version
  const {
    root: fsRoot,
    listContainer: fsListContainer,
    input: fsInput,
  } = uiOptions.fsRootRender(
    createElement, options,
    () => uiOptions.hideFullscreen(fsRoot, fsListContainer, uiOptions.fsContainer, options),
  );

  fsInput.addEventListener('input', createInputListener(fsRoot, fsListContainer, searcher, options));

  // Initial state is blank
  fsListContainer.appendChild(uiOptions.fsBlankRender(createElement, options));
  // --------------------------------------------------

  // --------------------------------------------------
  // Input element option handling
  let dropdownListContainer;
  const { input } = uiOptions;
  if (input && (uiOptions.mode === UiMode.Auto || uiOptions.mode === UiMode.Dropdown)) {
    // Auto / Dropdown

    const parent = input.parentElement;
    input.remove();
    const {
      dropdownRoot, dropdownListContainer: dropdownListContainerr,
    } = uiOptions.dropdownRootRender(createElement, options, input);
    dropdownListContainer = dropdownListContainerr;
    parent.appendChild(dropdownRoot);

    input.addEventListener(
      'input',
      createInputListener(dropdownRoot, dropdownListContainer, searcher, options),
    );

    function refreshDropdown() {
      uiOptions.hideDropdown(dropdownRoot, dropdownListContainer, options);
      if (document.activeElement === input) {
        uiOptions.showDropdown(dropdownRoot, dropdownListContainer, options);
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
          uiOptions.hideDropdown(dropdownRoot, dropdownListContainer, options);
        } else {
          uiOptions.hideFullscreen(fsRoot, fsListContainer, uiOptions.fsContainer, options);
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
          uiOptions.hideDropdown(dropdownRoot, dropdownListContainer, options);
        }, 100);
      }
    });

    input.addEventListener('focus', () => {
      if (useDropdown(uiOptions)) {
        uiOptions.showDropdown(dropdownRoot, dropdownListContainer, options);
        return;
      }

      // When using 'auto' mode, may still be using fullscreen UI
      uiOptions.showFullscreen(fsRoot, fsListContainer, uiOptions.fsContainer, options);
    });
  } else if (input && uiOptions.mode === UiMode.Fullscreen) {
    // Fullscreen-only mode
    input.addEventListener('focus', () => {
      uiOptions.showFullscreen(fsRoot, fsListContainer, uiOptions.fsContainer, options);
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

  // Not attached, just used for isEqualNode
  const loadingIndicator = uiOptions.loadingIndicatorRender(createElement, options);

  function keydownListener(ev: KeyboardEvent) {
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter'].includes(ev.key)) {
      return;
    }

    let listContainer: HTMLElement;

    let scrollListContainer = (targetEl: any) => {
      listContainer.scrollTo({ top: targetEl.offsetTop - listContainer.offsetTop - 30 });
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

    function currentFocused() {
      return listContainer.querySelector('.focus');
    }

    function focusEl(el: Element) {
      el.classList.add('focus');
      el.setAttribute('aria-selected', 'true');
      el.setAttribute('id', 'morsels-list-selected');
      scrollListContainer(el);
    }

    function unfocusEl(el: Element) {
      el.classList.remove('focus');
      el.removeAttribute('aria-selected');
      el.removeAttribute('id');
    }

    if (ev.key === 'ArrowDown') {
      const focusedItem = currentFocused();
      if (focusedItem) {
        if (focusedItem.nextElementSibling
          && !loadingIndicator.isEqualNode(focusedItem.nextElementSibling)) {
          unfocusEl(focusedItem);
          focusEl(focusedItem.nextElementSibling);
        }
      } else {
        focusEl(listContainer.firstElementChild);
      }
    } else if (ev.key === 'ArrowUp') {
      const focusedItem = currentFocused();
      if (focusedItem && focusedItem.previousElementSibling) {
        unfocusEl(focusedItem);
        focusEl(focusedItem.previousElementSibling);
      }
    } if (ev.key === 'Home' || ev.key === 'End') {
      const focusedItem = currentFocused();
      if (focusedItem) {
        unfocusEl(focusedItem);
      }

      let elToFocus = ev.key === 'Home' ? listContainer.firstElementChild : listContainer.lastElementChild;
      if (elToFocus && loadingIndicator.isEqualNode(elToFocus)) {
        elToFocus = elToFocus.previousElementSibling;
      }

      if (elToFocus) {
        focusEl(elToFocus);
      }
    } else if (ev.key === 'Enter') {
      const focusedItem = currentFocused();
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
    showFullscreen: () => {
      uiOptions.showFullscreen(fsRoot, fsListContainer, uiOptions.fsContainer, options);
    },
    hideFullscreen: () => {
      uiOptions.hideFullscreen(fsRoot, fsListContainer, uiOptions.fsContainer, options);
    },
  };
}

export default initMorsels;
