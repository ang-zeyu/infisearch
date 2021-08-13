import './styles/search.css';

import { Searcher, Query } from '@morsels/search-lib';
import createElement from './utils/dom';
import transformResults from './searchResultTransform';
import { SearchUiOptions } from './SearchUiOptions';

let query: Query;

let usePortal = false;

function hide(container: HTMLElement): void {
  if (usePortal) {
    container.parentElement.style.display = 'none';
  } else {
    (container.previousSibling as HTMLElement).style.display = 'none';
    container.style.display = 'none';
  }
}

function show(container: HTMLElement): void {
  if (usePortal) {
    container.parentElement.style.display = 'block';
  } else {
    (container.previousSibling as HTMLElement).style.display = 'block';
    container.style.display = 'block';
  }
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

function prepareOptions(options: SearchUiOptions, isMobile: boolean) {
  if (!('useQueryTermExpansion' in options.searcherOptions)) {
    options.searcherOptions.useQueryTermExpansion = true;
  }

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

  usePortal = isMobile;

  if (!('manualPortalControl' in options.render)) {
    options.render.manualPortalControl = false;
  }

  if (!('portalTo' in options.render)) {
    // eslint-disable-next-line prefer-destructuring
    options.render.portalTo = document.getElementsByTagName('body')[0];
  }

  if (!('portalInputRender' in options.render)) {
    options.render.portalInputRender = (h) => h(
      'input', { class: 'morsels-portal-input', type: 'text' },
    ) as HTMLInputElement;
  }

  if (!('inputWrapperRender' in options.render)) {
    options.render.inputWrapperRender = (h, inputEl, portalCloseHandler) => {
      const portalCloseButton = portalCloseHandler
        ? h('button', { class: 'morsels-input-close-portal' }, 'X')
        : '';
      if (portalCloseButton) {
        portalCloseButton.addEventListener('click', portalCloseHandler);
      }

      return h(
        'div', { class: `morsels-input-wrapper${portalCloseHandler ? ' morsels-input-wrapper-portal' : ''}` },
        inputEl,
        portalCloseButton,
        h('div', { class: 'morsels-input-dropdown-separator', style: 'display: none;' }),
      );
    };
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

function initMorsels(options: SearchUiOptions): () => void {
  const isMobile = window.matchMedia('only screen and (max-width: 1024px)').matches;
  prepareOptions(options, isMobile);

  const searcher = new Searcher(options.searcherOptions);

  let inputTimer: any = -1;
  const inputListener = (listContainer: HTMLElement) => (ev) => {
    const query = (ev.target as HTMLInputElement).value;

    if (query.length) {
      clearTimeout(inputTimer);
      inputTimer = setTimeout(() => {
        if (isUpdating) {
          nextUpdate = () => update(query, listContainer, searcher, options);
        } else {
          isUpdating = true;
          update(query, listContainer, searcher, options);
        }
      }, 200);
    } else if (!usePortal) {
      clearTimeout(inputTimer);
      if (isUpdating) {
        nextUpdate = () => {
          hide(listContainer);
          isUpdating = false;
        };
      } else {
        hide(listContainer);
      }
    }
  };

  // Fullscreen portal-ed version
  const mobileInput: HTMLInputElement = options.render.portalInputRender(createElement);
  const mobileListContainer = options.render.listRender(createElement);
  const mobileInputWrapper = options.render.inputWrapperRender(
    createElement, mobileInput, () => hide(mobileListContainer),
  );
  mobileInputWrapper.appendChild(mobileListContainer);
  mobileInputWrapper.style.display = 'none';

  let didAttachPortalContainer = false;
  const showPortalUI = () => {
    if (!didAttachPortalContainer) {
      didAttachPortalContainer = true;
      options.render.portalTo.appendChild(mobileInputWrapper);
      mobileInput.addEventListener('input', inputListener(mobileListContainer));
    }

    usePortal = true;
    show(mobileListContainer);
    mobileInput.focus();
  };

  // Dropdown version
  const input = document.getElementById(options.inputId);
  if (input) {
    const parent = input.parentElement;
    input.remove();
    const listContainer = options.render.listRender(createElement);
    const inputWrapper = options.render.inputWrapperRender(createElement, input);
    inputWrapper.appendChild(listContainer);
    parent.appendChild(inputWrapper);

    input.addEventListener('input', inputListener(listContainer));

    input.addEventListener('blur', () => {
      if (usePortal) {
        return;
      }

      setTimeout(() => {
        let activeEl = document.activeElement;
        while (activeEl) {
          activeEl = activeEl.parentElement;
          if (activeEl === listContainer) {
            input.focus();
            return;
          }
        }
        hide(listContainer);
      }, 100);
    });

    input.addEventListener('focus', () => {
      if (usePortal) {
        if (!options.render.manualPortalControl) {
          showPortalUI();
        }
      } else if (listContainer.childElementCount) {
        show(listContainer);
      }
    });
  }

  if (!options.render.manualPortalControl && input) {
    let debounce;
    window.addEventListener('resize', () => {
      clearTimeout(debounce);
      debounce = setTimeout(() => {
        if (window.matchMedia('only screen and (max-width: 1024px)').matches) {
          usePortal = true;
        }
      }, 200);
    });
  }

  return showPortalUI;
}

export default initMorsels;
