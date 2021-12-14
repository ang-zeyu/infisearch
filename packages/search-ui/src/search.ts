import { computePosition, size, offset, flip } from '@floating-ui/dom';

import './styles/search.css';

import { Query, Searcher } from '@morsels/search-lib';
import transformResults, { resultsRender } from './searchResultTransform';
import { SearchUiOptions, UiMode, UiOptions } from './SearchUiOptions';
import createElement from './utils/dom';
import { parseURL } from './utils/url';

let currQuery: Query;

let isMobileSizeGlobal = false;

let dropdownShown = false;
let fullscreenShown = false;

let isUpdating = false;
let nextUpdate: () => any;
async function update(
  queryString: string,
  root: HTMLElement,
  listContainer: HTMLElement,
  searcher: Searcher,
  options: SearchUiOptions,
): Promise<void> {
  try {
    // const now = performance.now();

    if (currQuery) {
      currQuery.free();
    }

    currQuery = await searcher.getQuery(queryString);

    // console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    await transformResults(currQuery, searcher.morselsConfig, true, listContainer, options);

    root.scrollTo({ top: 0 });
    listContainer.scrollTo({ top: 0 });
  } catch (ex) {
    console.error(ex);
    listContainer.innerHTML = '<div class="morsels-error"></div>';
    throw ex;
  } finally {
    if (nextUpdate) {
      const nextUpdateTemp = nextUpdate;
      nextUpdate = undefined;
      await nextUpdateTemp();
    } else {
      isUpdating = false;
    }
  }
}

function useDropdown(uiOptions: UiOptions): boolean {
  return (uiOptions.mode === UiMode.Auto && !isMobileSizeGlobal)
      || uiOptions.mode === UiMode.Dropdown;
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

  if (typeof uiOptions.fullscreenContainer === 'string') {
    uiOptions.fullscreenContainer = document.getElementById(uiOptions.fullscreenContainer) as HTMLElement;
  }

  if (!('dropdownAlignment' in uiOptions)) {
    uiOptions.dropdownAlignment = 'bottom-end';
  }

  if (!uiOptions.fullscreenContainer) {
    uiOptions.fullscreenContainer = document.getElementsByTagName('body')[0] as HTMLElement;
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
        strategy: 'fixed',
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

  const showFullscreen = uiOptions.showFullscreen || ((root, listContainer, fullscreenContainer) => {
    fullscreenContainer.appendChild(root);
    const input: HTMLInputElement = root.querySelector('input.morsels-fs-input');
    if (input) {
      input.focus();
    }
  });
  uiOptions.showFullscreen = (...args) => {
    showFullscreen(...args);
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

  uiOptions.dropdownRootRender = uiOptions.dropdownRootRender || ((h, opts, inputEl) => {
    const root = h('div', { class: 'morsels-root' }, inputEl);

    root.appendChild(h('div', {
      class: `morsels-input-dropdown-separator ${uiOptions.dropdownAlignment}`,
      style: 'display: none;',
    }));

    const listContainer = h('ul', {
      class: 'morsels-list',
      style: 'display: none;',
    });
    root.appendChild(listContainer);

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

    const inputEl = h(
      'input', { class: 'morsels-fs-input', type: 'text', placeholder: 'Search...' },
    ) as HTMLInputElement;

    const buttonEl = h('button', { class: 'morsels-input-close-fs' });
    buttonEl.onclick = fsCloseHandler;

    innerRoot.appendChild(h('div',
      { class: 'morsels-fs-input-button-wrapper' },
      inputEl,
      buttonEl));

    const listContainer = h('ul', { class: 'morsels-list' });
    innerRoot.appendChild(listContainer);

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

  uiOptions.noResultsRender = uiOptions.noResultsRender
      || ((h) => h('div', { class: 'morsels-no-results' }));

  uiOptions.fsBlankRender = uiOptions.fsBlankRender
      || ((h) => h('div', { class: 'morsels-fs-blank' }));

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
      'li', { class: 'morsels-list-item' },
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

  let inputTimer: any = -1;
  let isFirstQueryFromBlank = true;
  const inputListener = (root: HTMLElement, listContainer: HTMLElement) => (ev) => {
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

        if (isUpdating) {
          nextUpdate = () => update(query, root, listContainer, searcher, options);
        } else {
          isUpdating = true;
          update(query, root, listContainer, searcher, options);
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

        isUpdating = false;
        isFirstQueryFromBlank = true;
      };

      if (isUpdating) {
        nextUpdate = reset;
      } else {
        reset();
      }
    }
  };

  // --------------------------------------------------
  // Fullscreen version
  const {
    root: fsRoot,
    listContainer: fsListContainer,
    input: fsInput,
  } = uiOptions.fsRootRender(
    createElement, options,
    () => uiOptions.hideFullscreen(fsRoot, fsListContainer, uiOptions.fullscreenContainer, options),
  );

  fsInput.addEventListener('input', inputListener(fsRoot, fsListContainer));

  // Initial state is blank
  fsListContainer.appendChild(uiOptions.fsBlankRender(createElement, options));
  // --------------------------------------------------

  // --------------------------------------------------
  // Input element option handling
  // Applicable for all modes except the UiMode.Fullscreen which has its own input
  let dropdownListContainer;
  const { input } = uiOptions;
  if (input && uiOptions.mode !== UiMode.Target) {
    // Auto / Dropdown

    const parent = input.parentElement;
    input.remove();
    const {
      dropdownRoot, dropdownListContainer: dropdownListContainerr,
    } = uiOptions.dropdownRootRender(createElement, options, input);
    dropdownListContainer = dropdownListContainerr;
    parent.appendChild(dropdownRoot);

    input.addEventListener('input', inputListener(dropdownRoot, dropdownListContainer));

    if (uiOptions.mode === UiMode.Auto || uiOptions.mode === UiMode.Dropdown) {
      let debounce;
      window.addEventListener('resize', () => {
        if (uiOptions.mode === UiMode.Dropdown) {
          uiOptions.hideDropdown(dropdownRoot, dropdownListContainer, options);
          return;
        }

        clearTimeout(debounce);
        debounce = setTimeout(() => {
          const newIsMobileSize = isMobileDevice();

          if (isMobileSizeGlobal !== newIsMobileSize) {
            isMobileSizeGlobal = newIsMobileSize;
            if (isMobileSizeGlobal) {
              uiOptions.hideDropdown(dropdownRoot, dropdownListContainer, options);
            } else {
              uiOptions.hideFullscreen(fsRoot, fsListContainer, uiOptions.fullscreenContainer, options);
            }
          }
        }, 250);
      });
    }

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

      // useFullscreen
      uiOptions.showFullscreen(fsRoot, fsListContainer, uiOptions.fullscreenContainer, options);
    });
  } else if (input && uiOptions.mode === UiMode.Target) {
    // Target
    input.addEventListener('input', inputListener(uiOptions.target, uiOptions.target));
  }
  // --------------------------------------------------

  const loadingIndicator = uiOptions.loadingIndicatorRender(createElement, options);

  // Keyboard Events
  document.addEventListener('keydown', (ev) => {
    if (!['ArrowDown', 'ArrowUp', 'Enter'].includes(ev.key)) {
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

    if (ev.key === 'ArrowDown') {
      const currentFocusedResult = listContainer.querySelector('.focus');
      if (currentFocusedResult) {
        if (currentFocusedResult.nextElementSibling
          && !loadingIndicator.isEqualNode(currentFocusedResult.nextElementSibling)) {
          currentFocusedResult.classList.remove('focus');
          currentFocusedResult.nextElementSibling.classList.add('focus');
          scrollListContainer(currentFocusedResult.nextElementSibling);
        }
      } else {
        listContainer.firstElementChild.classList.add('focus');
      }
    } else if (ev.key === 'ArrowUp') {
      const currentFocusedResult = listContainer.querySelector('.focus');
      if (currentFocusedResult && currentFocusedResult.previousElementSibling) {
        currentFocusedResult.classList.remove('focus');
        currentFocusedResult.previousElementSibling.classList.add('focus');
        scrollListContainer(currentFocusedResult.previousElementSibling);
      }
    } else if (ev.key === 'Enter') {
      const currentFocusedResult = listContainer.querySelector('.focus');
      if (currentFocusedResult) {
        const link = currentFocusedResult.querySelector('a[href]');
        if (link) {
          window.location.href = link.getAttribute('href');
        }
      }
    }
  });

  return {
    showFullscreen: () => {
      uiOptions.showFullscreen(fsRoot, fsListContainer, uiOptions.fullscreenContainer, options);
    },
    hideFullscreen: () => {
      uiOptions.hideFullscreen(fsRoot, fsListContainer, uiOptions.fullscreenContainer, options);
    },
  };
}

export default initMorsels;
