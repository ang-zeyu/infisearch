import './styles/search.css';

import { Searcher, Query } from '@morsels/search-lib';
import createElement from './utils/dom';
import transformResults from './searchResultTransform';
import { SearchUiOptions } from './SearchUiOptions';

let query: Query;

function hide(container: HTMLElement): void {
  (container.previousSibling as HTMLElement).style.display = 'none';
  container.style.display = 'none';
}

function show(container: HTMLElement): void {
  (container.previousSibling as HTMLElement).style.display = 'block';
  container.style.display = 'block';
}

let isUpdating = false;
let nextUpdate: () => any;
async function update(
  queryString: string,
  container: HTMLElement,
  searcher: Searcher,
  options: SearchUiOptions,
): Promise<void> {
  try {
    const now = performance.now();

    if (query) {
      query.free();
    }

    container.innerHTML = '';
    container.appendChild(options.render.loadingIndicatorRender(createElement));
    show(container);

    query = await searcher.getQuery(queryString);

    console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    await transformResults(query, searcher.morselsConfig, true, container, options);
  } catch (ex) {
    container.innerHTML = ex.message;
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

function prepareOptions(options: SearchUiOptions) {
  if (!('useQueryTermExpansion' in options.searcherOptions)) {
    options.searcherOptions.useQueryTermExpansion = true;
  }

  const isMobile = window.matchMedia('only screen and (max-width: 1024px)').matches;
  if (!('useQueryTermProximity' in options.searcherOptions)) {
    options.searcherOptions.useQueryTermProximity = !isMobile;
  }

  if (!('inputId' in options)) {
    options.inputId = 'morsels-search';
  }

  if (!('resultsPerPage' in options)) {
    options.resultsPerPage = isMobile ? 8 : 10;
  }

  if (!('sourceFilesUrl' in options)) {
    options.sourceFilesUrl = '';
  }

  if (!('render' in options)) {
    options.render = {};
  }

  if (!('inputWrapperRender' in options.render)) {
    options.render.inputWrapperRender = (h, inputEl) => h(
      'div', { class: 'morsels-input-wrapper' },
      inputEl,
      h('div', { class: 'morsels-input-dropdown-separator', style: 'display: none;' }),
    );
  }

  if (!('noResultsRender' in options.render)) {
    options.render.noResultsRender = (h) => h('div', { class: 'morsels-no-results' }, 'no results found');
  }

  if (!('listRender' in options.render)) {
    options.render.listRender = (h) => h('ul', { class: 'morsels-dropdown', style: 'display: none;' });
  }

  if (!('listItemRender' in options.render)) {
    options.render.listItemRender = (h, fullLink, title, bodies) => {
      const linkEl = h('a', { class: 'morsels-link' },
        h('div', { class: 'morsels-title' }, title),
        ...bodies);
      if (fullLink) {
        linkEl.setAttribute('href', fullLink);
      }

      return h(
        'li', { class: 'morsels-dropdown-item' },
        linkEl,
      );
    };
  }

  if (!('loadingIndicatorRender' in options.render)) {
    options.render.loadingIndicatorRender = (h) => h('span', { class: 'morsels-loading-indicator' });
  }

  if (!('highlightRender' in options.render)) {
    options.render.highlightRender = (h, matchedPart) => h(
      'span', { class: 'morsels-highlight' }, matchedPart,
    );
  }

  if (!('headingBodyRender' in options.render)) {
    options.render.headingBodyRender = (h, heading, bodyHighlights, href) => {
      const el = h('a', { class: 'morsels-heading-body' },
        h('div', { class: 'morsels-heading' }, heading),
        h('div', { class: 'morsels-bodies' },
          h('div', { class: 'morsels-body' }, ...bodyHighlights)));
      if (href) {
        el.setAttribute('href', href);
      }
      return el;
    };
  }

  if (!('bodyOnlyRender' in options.render)) {
    options.render.bodyOnlyRender = (h, bodyHighlights) => h(
      'div', { class: 'morsels-body' }, ...bodyHighlights,
    );
  }

  if (!('termInfoRender' in options.render)) {
    options.render.termInfoRender = (h, misspelledTerms, correctedTerms, expandedTerms) => {
      const returnVal: HTMLElement[] = [];
      const correctedTermsContainer = h('div', { class: 'morsels-suggestion-container-corrected' });

      if (expandedTerms.length) {
        returnVal.push(
          h('div', { class: 'morsels-suggestion-container-expanded' },
            h('div', { class: 'morsels-suggestion-content' },
              'Also searched for... ',
              h('small', {}, '(add a space to the last term to finalise the search)'),
              h('br', {}),
              ...expandedTerms.map((expandedTerm, idx) => (idx === 0 ? '' : h(
                'span', { class: 'morsels-suggestion-expanded' }, `${expandedTerm} `,
              ))))),
        );
      }

      if (misspelledTerms.length) {
        correctedTermsContainer.prepend(
          h('div', { class: 'morsels-suggestion-content' },
            'Could not find any matches for',
            ...misspelledTerms.map((term) => h(
              'span', { class: 'morsels-suggestion-wrong' }, ` "${term}"`,
            )),
            correctedTerms.length ? ', searched for: ' : '',
            ...correctedTerms.map((correctedTerm) => h(
              'span', { class: 'morsels-suggestion-corrected' }, `${correctedTerm} `,
            ))),
        );
        returnVal.push(correctedTermsContainer);
      }

      return returnVal;
    };
  }
}

function initMorsels(options: SearchUiOptions): void {
  prepareOptions(options);

  const input = document.getElementById(options.inputId);
  if (!input) {
    return;
  }

  const parent = input.parentElement;
  input.remove();
  const inputWrapper = options.render.inputWrapperRender(createElement, input);
  const container = options.render.listRender(createElement);
  inputWrapper.appendChild(container);
  parent.appendChild(inputWrapper);

  const searcher = new Searcher(options.searcherOptions);

  let inputTimer: any = -1;
  input.addEventListener('input', (ev) => {
    const query = (ev.target as HTMLInputElement).value;

    if (query.length) {
      clearTimeout(inputTimer);
      inputTimer = setTimeout(() => {
        if (isUpdating) {
          nextUpdate = () => update(query, container, searcher, options);
        } else {
          isUpdating = true;
          update(query, container, searcher, options);
        }
      }, 200);
    } else {
      clearTimeout(inputTimer);
      if (isUpdating) {
        nextUpdate = () => {
          hide(container);
          isUpdating = false;
        };
      } else {
        hide(container);
      }
    }
  });

  const blurListener = () => {
    setTimeout(() => {
      let activeEl = document.activeElement;
      while (activeEl) {
        activeEl = activeEl.parentElement;
        if (activeEl === container) {
          input.focus();
          return;
        }
      }
      hide(container);
    }, 100);
  };

  input.addEventListener('blur', blurListener);

  input.addEventListener('focus', () => {
    if (container.childElementCount) {
      show(container);
    }
  });
}

export default initMorsels;
