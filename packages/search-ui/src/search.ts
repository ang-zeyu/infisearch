import './styles/search.css';

import { Searcher, Query } from '@morsels/search-lib';
import createElement from './utils/dom';
import transformResults from './searchResultTransform';
import { SearchUiOptions } from './SearchUiOptions';

let query: Query;

let autoPortalControlFlag = false;

let isUpdating = false;
let nextUpdate: () => any;
async function update(
  queryString: string,
  root: HTMLElement,
  listContainer: HTMLElement,
  forPortal: boolean,
  searcher: Searcher,
  options: SearchUiOptions,
): Promise<void> {
  try {
    const now = performance.now();

    if (query) {
      query.free();
    }

    listContainer.innerHTML = '';
    listContainer.appendChild(options.render.loadingIndicatorRender(createElement));
    options.render.show(root, forPortal);

    query = await searcher.getQuery(queryString);

    console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    await transformResults(query, searcher.morselsConfig, true, listContainer, options);
  } catch (ex) {
    listContainer.innerHTML = ex.message;
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

  autoPortalControlFlag = isMobile;

  options.render.manualPortalControl = options.render.manualPortalControl || false;

  options.render.portalTo = options.render.portalTo || document.getElementsByTagName('body')[0];

  options.render.show = options.render.show || ((root, forPortal) => {
    if (forPortal) {
      root.style.display = 'block';
    } else {
      (root.lastElementChild as HTMLElement).style.display = 'block';
      (root.lastElementChild.previousSibling as HTMLElement).style.display = 'block';
    }
  });

  options.render.hide = options.render.hide || ((root, forPortal) => {
    if (forPortal) {
      root.style.display = 'none';
    } else {
      (root.lastElementChild as HTMLElement).style.display = 'none';
      (root.lastElementChild.previousSibling as HTMLElement).style.display = 'none';
    }
  });

  options.render.rootRender = options.render.rootRender || ((h, inputEl, portalCloseHandler) => {
    const portalCloseButton = portalCloseHandler
      ? h('button', { class: 'morsels-input-close-portal' }, 'X')
      : '';
    if (portalCloseButton) {
      portalCloseButton.addEventListener('click', portalCloseHandler);
    }

    const dropdownSeparator = portalCloseHandler
      ? ''
      : h('div', { class: 'morsels-input-dropdown-separator', style: 'display: none;' });

    const listContainer = h('ul', {
      class: 'morsels-list',
      style: portalCloseHandler ? '' : 'display: none;',
    });

    return {
      root: h(
        'div',
        {
          class: `morsels-input-wrapper${portalCloseHandler ? ' morsels-input-wrapper-portal' : ''}`,
        },
        inputEl,
        portalCloseButton,
        dropdownSeparator,
        listContainer,
      ),
      listContainer,
    };
  });

  options.render.portalInputRender = options.render.portalInputRender || ((h) => h(
    'input', { class: 'morsels-portal-input', type: 'text' },
  ) as HTMLInputElement);

  options.render.listItemRender = options.render.listItemRender || ((h, fullLink, title, bodies) => {
    const linkEl = h('a', { class: 'morsels-link' },
      h('div', { class: 'morsels-title' }, title),
      ...bodies);
    if (fullLink) {
      linkEl.setAttribute('href', fullLink);
    }

    return h(
      'li', { class: 'morsels-list-item' },
      linkEl,
    );
  });

  options.render.loadingIndicatorRender = options.render.loadingIndicatorRender
        || ((h) => h('span', { class: 'morsels-loading-indicator' }));

  options.render.highlightRender = options.render.highlightRender || ((h, matchedPart) => h(
    'span', { class: 'morsels-highlight' }, matchedPart,
  ));

  options.render.headingBodyRender = options.render.headingBodyRender || ((
    h, heading, bodyHighlights, href,
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

  options.render.bodyOnlyRender = options.render.bodyOnlyRender || ((h, bodyHighlights) => h(
    'div', { class: 'morsels-body' }, ...bodyHighlights,
  ));

  options.render.termInfoRender = options.render.termInfoRender || ((
    h, misspelledTerms, correctedTerms, expandedTerms,
  ) => {
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
  });

  options.render.noResultsRender = options.render.noResultsRender
        || ((h) => h('div', { class: 'morsels-no-results' }, 'no results found'));
}

function initMorsels(options: SearchUiOptions): () => void {
  const isMobile = window.matchMedia('only screen and (max-width: 1024px)').matches;
  prepareOptions(options, isMobile);

  const searcher = new Searcher(options.searcherOptions);

  let inputTimer: any = -1;
  const inputListener = (root: HTMLElement, listContainer: HTMLElement, forPortal: boolean) => (ev) => {
    const query = (ev.target as HTMLInputElement).value;

    if (query.length) {
      clearTimeout(inputTimer);
      inputTimer = setTimeout(() => {
        if (isUpdating) {
          nextUpdate = () => update(query, root, listContainer, forPortal, searcher, options);
        } else {
          isUpdating = true;
          update(query, root, listContainer, forPortal, searcher, options);
        }
      }, 200);
    } else if (!autoPortalControlFlag) {
      clearTimeout(inputTimer);
      if (isUpdating) {
        nextUpdate = () => {
          options.render.hide(root, forPortal);
          isUpdating = false;
        };
      } else {
        options.render.hide(root, forPortal);
      }
    }
  };

  // Fullscreen portal-ed version
  const mobileInput: HTMLInputElement = options.render.portalInputRender(createElement);
  const { root: portalRoot, listContainer: portalListContainer } = options.render.rootRender(
    createElement, mobileInput, () => options.render.hide(portalRoot, true),
  );
  portalRoot.style.display = 'none';

  let didAttachPortalContainer = false;
  const showPortalUI = () => {
    if (!didAttachPortalContainer) {
      didAttachPortalContainer = true;
      options.render.portalTo.appendChild(portalRoot);
      mobileInput.addEventListener('input', inputListener(portalRoot, portalListContainer, true));
    }

    autoPortalControlFlag = true;
    options.render.show(portalRoot, true);
    mobileInput.focus();
  };

  // Dropdown version
  const input = document.getElementById(options.inputId);
  if (input) {
    const parent = input.parentElement;
    input.remove();
    const {
      root, listContainer,
    } = options.render.rootRender(createElement, input);
    parent.appendChild(root);

    input.addEventListener('input', inputListener(root, listContainer, false));

    input.addEventListener('blur', () => {
      if (autoPortalControlFlag) {
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
        options.render.hide(root, false);
      }, 100);
    });

    input.addEventListener('focus', () => {
      if (autoPortalControlFlag) {
        if (!options.render.manualPortalControl) {
          showPortalUI();
        }
      } else if (listContainer.childElementCount) {
        options.render.show(root, false);
      }
    });
  }

  if (!options.render.manualPortalControl && input) {
    let debounce;
    window.addEventListener('resize', () => {
      clearTimeout(debounce);
      debounce = setTimeout(() => {
        if (window.matchMedia('only screen and (max-width: 1024px)').matches) {
          autoPortalControlFlag = true;
        }
      }, 200);
    });
  }

  return showPortalUI;
}

export default initMorsels;
