import { computePosition, size, flip, arrow } from '@floating-ui/dom';

import { resultsRender } from '../searchResultTransform';
import { SearchUiOptions, UiMode } from '../SearchUiOptions';
import { setCombobox, setInputAria } from '../utils/aria';
import { parseURL } from '../utils/url';
import { LOADING_INDICATOR_ID } from '../utils/dom';

export function prepareOptions(options: SearchUiOptions, isMobile: boolean) {
  // ------------------------------------------------------------
  // Search Lib Options
  
  options.searcherOptions = options.searcherOptions || ({} as any);
  
  if (!('url' in options.searcherOptions)) {
    throw new Error('Mandatory url parameter not specified');
  } else if (!options.searcherOptions.url.endsWith('/')) {
    options.searcherOptions.url += '/';
  }
  
  if (!('numberOfExpandedTerms' in options.searcherOptions)) {
    options.searcherOptions.numberOfExpandedTerms = 3;
  }
  
  if (!('useQueryTermProximity' in options.searcherOptions)) {
    options.searcherOptions.useQueryTermProximity = !isMobile;
  }
  
  if (!('resultLimit' in options.searcherOptions)) {
    options.searcherOptions.resultLimit = null; // unlimited
  }
  
  // ------------------------------------------------------------
  // Ui Options
  
  options.uiOptions = options.uiOptions || ({} as any);
  const { uiOptions } = options;
  
  if (uiOptions.sourceFilesUrl && !uiOptions.sourceFilesUrl.endsWith('/')) {
    uiOptions.sourceFilesUrl += '/';
  }
  
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
  
  uiOptions.preprocessQuery = uiOptions.preprocessQuery || ((q) => q);
  
  uiOptions.dropdownAlignment = uiOptions.dropdownAlignment || 'bottom-end';
  
  if (typeof uiOptions.fsContainer === 'string') {
    uiOptions.fsContainer = document.getElementById(uiOptions.fsContainer) as HTMLElement;
  }
  uiOptions.fsContainer = uiOptions.fsContainer || document.getElementsByTagName('body')[0] as HTMLElement;
  
  uiOptions.resultsPerPage = uiOptions.resultsPerPage || 8;
  
  uiOptions.showDropdown = uiOptions.showDropdown || ((root, listContainer) => {
    if (listContainer.childElementCount) {
      const innerRoot = root.lastElementChild as HTMLElement;
      const caret = innerRoot.firstElementChild as HTMLElement;
      innerRoot.style.display = 'block';
      computePosition(root, innerRoot, {
        placement: uiOptions.dropdownAlignment,
        middleware: [
          flip({
            padding: 10,
            mainAxis: false,
          }),
          size({
            apply({ availableWidth, availableHeight }) {
              Object.assign(listContainer.style, {
                maxWidth: `min(${availableWidth}px, var(--morsels-dropdown-max-width))`,
                maxHeight: `min(${availableHeight}px, var(--morsels-dropdown-max-height))`,
              });
            },
            padding: 10,
          }),
          arrow({
            element: caret,
          }),
        ],
      }).then(({ x, y, middlewareData }) => {
        Object.assign(innerRoot.style, {
          left: `${x}px`,
          top: `${y}px`,
        });
  
        const { x: arrowX } = middlewareData.arrow;
        Object.assign(caret.style, {
          left: arrowX != null ? `${arrowX}px` : '',
        });
      });
    }
  });
  
  uiOptions.hideDropdown = uiOptions.hideDropdown || ((root) => {
    (root.lastElementChild as HTMLElement).style.display = 'none';
  });
  
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
  };
  
  uiOptions.hideFullscreen = uiOptions.hideFullscreen || ((root) => {
    // useFullscreen
    root.remove();
  });
  
  uiOptions.label = uiOptions.label || 'Search this site';
  uiOptions.fsPlaceholder = uiOptions.fsPlaceholder || 'Search this site...';
  uiOptions.fsCloseText = uiOptions.fsCloseText || 'Close';
  
  uiOptions.dropdownRootRender = uiOptions.dropdownRootRender || ((h, opts, inputEl) => {
    const listContainer = h('ul', {
      id: 'morsels-dropdown-list',
      class: 'morsels-list',
    });
    const root = h('div', { class: 'morsels-root' },
      inputEl,
      h('div',
        { class: 'morsels-inner-root', style: 'display: none;' },
        h('div', { class: 'morsels-input-dropdown-separator' }),
        listContainer,
      ),
    );
  
    setInputAria(inputEl, 'morsels-dropdown-list');
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
    buttonEl.onclick = (ev) => {
      ev.preventDefault();
      fsCloseHandler();
    };
  
    const listContainer = h('ul', {
      id: 'morsels-fs-list',
      class: 'morsels-list',
      'aria-labelledby': 'morsels-fs-label',
    });
    
    const innerRoot = h('div',
      { class: 'morsels-root morsels-fs-root' },
      h('form',
        { class: 'morsels-fs-input-button-wrapper' },
        h('label',
          { id: 'morsels-fs-label', for: 'morsels-fs-input', style: 'display: none' },
          opts.uiOptions.label,
        ),
        inputEl,
        buttonEl,
      ),
      listContainer,
    );
    innerRoot.onclick = (ev) => ev.stopPropagation();
    innerRoot.onmousedown = (ev) => ev.stopPropagation();
  
    setCombobox(innerRoot, listContainer, opts.uiOptions.label);
  
    const rootBackdropEl = h('div', { class: 'morsels-fs-backdrop' }, innerRoot);
    rootBackdropEl.onmousedown = () => {
      uiOptions.hideFullscreen(rootBackdropEl, listContainer, innerRoot, opts);
    };
    rootBackdropEl.onkeyup = (ev) => {
      if (ev.code === 'Escape') {
        ev.stopPropagation();
        uiOptions.hideFullscreen(rootBackdropEl, listContainer, innerRoot, opts);
      }
    };
  
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
  
  if (!uiOptions.loadingIndicatorRender) {
    uiOptions.loadingIndicatorRender = ((
      h, opts, isInitialising, wasResultsBlank,
    ) => {
      const loadingSpinner = h('span', { class: 'morsels-loading-indicator' });
      if (isInitialising) {
        const initialisingText = h('div', { class: 'morsels-initialising-text' }, '... Initialising ...');
        return h('div', { class: 'morsels-initialising' }, initialisingText, loadingSpinner);
      }
    
      if (!wasResultsBlank) {
        loadingSpinner.classList.add('morsels-loading-indicator-subsequent');
      }
    
      return loadingSpinner;
    });
  }
  const loadingIndicatorRenderer = uiOptions.loadingIndicatorRender;
  uiOptions.loadingIndicatorRender = (...args) => {
    const loadingIndicator = loadingIndicatorRenderer(...args);
    // Add an identifier for keyboard events (up / down / home / end)
    loadingIndicator.setAttribute(LOADING_INDICATOR_ID, 'true');
    return loadingIndicator;
  };
  
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
      h, opts, headingHighlights, bodyHighlights, href,
    ) => {
      const el = h('a', { class: 'morsels-heading-body' },
        h('div', { class: 'morsels-heading' }, ...headingHighlights),
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