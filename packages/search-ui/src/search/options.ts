import { resultsRender } from '../searchResultTransform';
import { Options, UiMode } from '../Options';
import { parseURL } from '../utils/url';
import { LOADING_INDICATOR_ID } from '../utils/dom';

// Undocumented option for mdBook
function appendSearchedTerms(
  opts: Options, fullLink: string, searchedTermsJSON: string,
) {
  const { addSearchedTerms } = opts.uiOptions.resultsRenderOpts;
  if (addSearchedTerms) {
    const fullLinkUrl = parseURL(fullLink);
    fullLinkUrl.searchParams.append(addSearchedTerms, searchedTermsJSON);
    return fullLinkUrl.toString();
  }
  return fullLink;
}

export function prepareOptions(options: Options) {
  // ------------------------------------------------------------
  // Search Lib Options
  
  options.searcherOptions = options.searcherOptions || ({} as any);

  const { searcherOptions } = options;
  
  if (!('url' in searcherOptions)) {
    throw new Error('Mandatory url parameter not specified');
  } else if (!searcherOptions.url.endsWith('/')) {
    searcherOptions.url += '/';
  }
  
  if (searcherOptions.url.startsWith('/')) {
    searcherOptions.url = window.location.origin + searcherOptions.url;
  }

  if (!('maxAutoSuffixSearchTerms' in searcherOptions)) {
    searcherOptions.maxAutoSuffixSearchTerms = 3;
  }

  if (!('maxSuffixSearchTerms' in searcherOptions)) {
    searcherOptions.maxSuffixSearchTerms = 5;
  }
  
  if (!('useQueryTermProximity' in searcherOptions)) {
    searcherOptions.useQueryTermProximity = true;
  }

  if (!('plLazyCacheThreshold' in searcherOptions)) {
    searcherOptions.plLazyCacheThreshold = 0;
  }
  
  if (!('resultLimit' in searcherOptions)) {
    searcherOptions.resultLimit = null; // unlimited
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
  
  uiOptions.resultsPerPage = uiOptions.resultsPerPage || 10;
  uiOptions.maxSubMatches = uiOptions.maxSubMatches || 2;
  
  uiOptions.label = uiOptions.label || 'Search this site';
  uiOptions.resultsLabel = uiOptions.resultsLabel || 'Site results';
  uiOptions.fsInputLabel = uiOptions.fsInputLabel || 'Search';
  uiOptions.fsPlaceholder = uiOptions.fsPlaceholder || 'Search this site';
  uiOptions.fsCloseText = uiOptions.fsCloseText || 'Close';
  if (!('fsScrollLock' in uiOptions)) {
    uiOptions.fsScrollLock = true;
  }
  
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
  
  uiOptions.headerRender = uiOptions.headerRender || ((h, opts, err, blank, queryParts) => {
    if (err) {
      return h('div', { class: 'morsels-header morsels-error' }, 'Oops! Something went wrong... 🙁');
    } else if (blank) {
      return h('div', { class: 'morsels-header morsels-fs-blank' }, 'Start Searching Above!');
    }

    return h('div', { class: 'morsels-header' }, `${queryParts.resultsTotal} results found`);
  });
  
  uiOptions.resultsRender = uiOptions.resultsRender || resultsRender;
  
  uiOptions.resultsRenderOpts = uiOptions.resultsRenderOpts || {};

  const { resultsRenderOpts } = uiOptions;
  
  resultsRenderOpts.listItemRender = resultsRenderOpts.listItemRender || ((
    h, opts, searchedTermsJSON, fullLink, title, matches,
  ) => {
    const bodies = matches.filter((r) => !r.headingMatches);
    const headings = matches.filter((r) => r.headingMatches);

    const mainLinkEl = h(
      'a', { class: 'morsels-title-link', role: 'option' },
      h('div', { class: 'morsels-title' }, title),
      ...bodies.map(({ bodyMatches }) => h(
        'div', { class: 'morsels-body' }, ...bodyMatches,
      )),
    );

    if (fullLink) {
      mainLinkEl.setAttribute('href', appendSearchedTerms(opts, fullLink, searchedTermsJSON));
    }

    const subOptions = headings.map(({ href, bodyMatches, headingMatches }) => {
      const el = h('a', { class: 'morsels-heading-link', role: 'option' },
        h('div', { class: 'morsels-heading' }, ...headingMatches),
        h('div', { class: 'morsels-body' }, ...bodyMatches));
      if (href) {
        el.setAttribute('href', appendSearchedTerms(opts, href, searchedTermsJSON));
      }
      return el;
    });
  
    return h(
      'div', { class: 'morsels-list-item', role: 'group', 'aria-label': title },
      mainLinkEl, ...subOptions,
    );
  });
  
  resultsRenderOpts.highlightRender = resultsRenderOpts.highlightRender || ((
    h, opts, matchedPart,
  ) => h(
    'span', { class: 'morsels-highlight' }, matchedPart,
  ));
  
  options.otherOptions = options.otherOptions || {};
}
