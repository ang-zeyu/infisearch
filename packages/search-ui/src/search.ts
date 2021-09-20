import './styles/search.css';

import { Searcher, Query } from '@morsels/search-lib';
import createElement from './utils/dom';
import transformResults, { resultsRender } from './searchResultTransform';
import { SearchUiOptions } from './SearchUiOptions';

let query: Query;

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
    // const now = performance.now();

    if (query) {
      query.free();
    }

    query = await searcher.getQuery(queryString);

    // console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    await transformResults(query, searcher.morselsConfig, true, listContainer, options);

    root.scrollTo({ top: 0 });
    listContainer.scrollTo({ top: 0 });
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
  if (!('numberOfExpandedTerms' in options.searcherOptions)) {
    options.searcherOptions.numberOfExpandedTerms = 3;
  }

  if (!('useQueryTermProximity' in options.searcherOptions)) {
    options.searcherOptions.useQueryTermProximity = !isMobile;
  }

  if (!('inputId' in options)) {
    options.inputId = 'morsels-search';
  }

  if (!('inputDebounce' in options)) {
    options.inputDebounce = isMobile ? 275 : 200;
  }

  if (!('render' in options)) {
    options.render = {};
  }

  if (!('enablePortal' in options.render)) {
    options.render.enablePortal = 'auto';
  }

  options.render.portalTo = options.render.portalTo || document.getElementsByTagName('body')[0];

  options.render.show = options.render.show || ((root, opts, forPortal) => {
    if (forPortal) {
      options.render.portalTo.appendChild(root);
      const input: HTMLInputElement = root.querySelector('input.morsels-portal-input');
      if (input) {
        input.focus();
      }
    } else {
      (root.lastElementChild as HTMLElement).style.display = 'block';
      (root.lastElementChild.previousSibling as HTMLElement).style.display = 'block';
    }
  });

  options.render.hide = options.render.hide || ((root, opts, forPortal) => {
    if (forPortal) {
      root.remove();
    } else {
      (root.lastElementChild as HTMLElement).style.display = 'none';
      (root.lastElementChild.previousSibling as HTMLElement).style.display = 'none';
    }
  });

  options.render.rootRender = options.render.rootRender || ((h, opts, inputEl) => {
    const root = h('div', { class: 'morsels-root' }, inputEl);

    root.appendChild(h('div', {
      class: `morsels-input-dropdown-separator ${opts.dropdownAlignment || 'right'}`,
      style: 'display: none;',
    }));

    const listContainer = h('ul', {
      class: `morsels-list ${opts.dropdownAlignment || 'right'}`,
      style: 'display: none;',
    });
    root.appendChild(listContainer);

    return {
      root,
      listContainer,
    };
  });

  options.render.portalRootRender = options.render.portalRootRender || ((
    h,
    opts,
    portalCloseHandler,
  ) => {
    const innerRoot = h('div', { class: 'morsels-root morsels-portal-root' });
    innerRoot.onclick = (ev) => ev.stopPropagation();

    const rootBackdropEl = h('div', { class: 'morsels-portal-backdrop' }, innerRoot);
    rootBackdropEl.onclick = () => rootBackdropEl.remove();

    const inputEl = h(
      'input', { class: 'morsels-portal-input', type: 'text' },
    ) as HTMLInputElement;

    const buttonEl = h('button', { class: 'morsels-input-close-portal' });
    buttonEl.onclick = portalCloseHandler;

    innerRoot.appendChild(h('div',
      { class: 'morsels-portal-input-button-wrapper' },
      inputEl,
      buttonEl));

    const listContainer = h('ul', { class: 'morsels-list' });
    innerRoot.appendChild(listContainer);

    return {
      root: rootBackdropEl,
      listContainer,
      input: inputEl,
    };
  });

  options.render.noResultsRender = options.render.noResultsRender
      || ((h) => h('div', { class: 'morsels-no-results' }));

  options.render.portalBlankRender = options.render.portalBlankRender
      || ((h) => h('div', { class: 'morsels-portal-blank' }));

  options.render.loadingIndicatorRender = options.render.loadingIndicatorRender
      || ((h) => h('span', { class: 'morsels-loading-indicator' }));

  options.render.termInfoRender = options.render.termInfoRender || (() => []);

  options.render.resultsRender = options.render.resultsRender || resultsRender;

  options.render.resultsRenderOpts = options.render.resultsRenderOpts || {};

  if (!('resultsPerPage' in options.render.resultsRenderOpts)) {
    options.render.resultsRenderOpts.resultsPerPage = 8;
  }

  options.render.resultsRenderOpts.listItemRender = options.render.resultsRenderOpts.listItemRender || ((
    h, opts, fullLink, title, bodies,
  ) => {
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

  options.render.resultsRenderOpts.headingBodyRender = options.render.resultsRenderOpts.headingBodyRender || ((
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

  options.render.resultsRenderOpts.bodyOnlyRender = options.render.resultsRenderOpts.bodyOnlyRender || ((
    h, opts, bodyHighlights,
  ) => h(
    'div', { class: 'morsels-body' }, ...bodyHighlights,
  ));

  options.render.resultsRenderOpts.highlightRender = options.render.resultsRenderOpts.highlightRender || ((
    h, opts, matchedPart,
  ) => h(
    'span', { class: 'morsels-highlight' }, matchedPart,
  ));

  options.render.opts = options.render.opts || {};
}

function initMorsels(options: SearchUiOptions): { show: () => void, hide: () => void } {
  const isMobile = window.matchMedia('only screen and (max-width: 1024px)').matches;
  prepareOptions(options, isMobile);

  const searcher = new Searcher(options.searcherOptions);

  let inputTimer: any = -1;
  let isFirstQueryFromBlank = true;
  const inputListener = (root: HTMLElement, listContainer: HTMLElement, forPortal: boolean) => (ev) => {
    const query = (ev.target as HTMLInputElement).value;

    clearTimeout(inputTimer);
    if (query.length) {
      inputTimer = setTimeout(() => {
        if (isFirstQueryFromBlank) {
          listContainer.innerHTML = '';
          listContainer.appendChild(options.render.loadingIndicatorRender(createElement, options.render.opts));
          if (!forPortal) {
            options.render.show(root, options.render.opts, forPortal);
          }
        }

        if (isUpdating) {
          nextUpdate = () => update(query, root, listContainer, forPortal, searcher, options);
        } else {
          isUpdating = true;
          update(query, root, listContainer, forPortal, searcher, options);
        }
        isFirstQueryFromBlank = false;
      }, options.inputDebounce);
    } else {
      const reset = () => {
        if (forPortal) {
          listContainer.innerHTML = '';
          listContainer.appendChild(options.render.portalBlankRender(createElement, options.render.opts));
        } else {
          options.render.hide(root, options.render.opts, forPortal);
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
  // Fullscreen portal-ed version
  const {
    root: portalRoot,
    listContainer: portalListContainer,
    input: portalInput,
  } = options.render.portalRootRender(
    createElement,
    options.render.opts,
    () => options.render.hide(portalRoot, options.render.opts, true),
  );

  portalInput.addEventListener('input', inputListener(portalRoot, portalListContainer, true));
  portalInput.addEventListener('keydown', (ev) => ev.stopPropagation());

  // Initial state is blank
  portalListContainer.appendChild(options.render.portalBlankRender(createElement, options.render.opts));
  // --------------------------------------------------

  // --------------------------------------------------
  // Dropdown version
  const input = options.inputId && document.getElementById(options.inputId);
  if (input) {
    const parent = input.parentElement;
    input.remove();
    const {
      root, listContainer,
    } = options.render.rootRender(createElement, options.render.opts, input);
    parent.appendChild(root);

    input.addEventListener('input', inputListener(root, listContainer, false));
    input.addEventListener('keydown', (ev) => ev.stopPropagation());

    input.addEventListener('blur', () => {
      if (options.render.enablePortal) {
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
        options.render.hide(root, options.render.opts, false);
      }, 100);
    });

    input.addEventListener('focus', () => {
      if (options.render.enablePortal) {
        options.render.show(portalRoot, options.render.opts, true);
      } else if (listContainer.childElementCount) {
        options.render.show(root, options.render.opts, false);
      }
    });

    if (options.render.enablePortal === 'auto') {
      options.render.enablePortal = isMobile;

      let debounce;
      window.addEventListener('resize', () => {
        clearTimeout(debounce);
        debounce = setTimeout(() => {
          const oldEnablePortal = options.render.enablePortal;
          options.render.enablePortal = window.matchMedia('only screen and (max-width: 1024px)').matches;

          if (oldEnablePortal !== options.render.enablePortal) {
            if (options.render.enablePortal) {
              options.render.hide(root, options.render.opts, false);
            } else {
              options.render.hide(portalRoot, options.render.opts, true);
            }
          }
        }, 250);
      });
    }
  }
  // --------------------------------------------------

  return {
    show: () => {
      options.render.show(portalRoot, options.render.opts, true);
    },
    hide: () => {
      options.render.hide(portalRoot, options.render.opts, true);
    },
  };
}

export default initMorsels;
