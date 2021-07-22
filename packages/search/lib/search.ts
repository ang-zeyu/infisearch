import * as escapeRegex from 'escape-string-regexp';

import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Query from './results/Query';
import { transformHtml, transformText } from './searchResultTransform';

const { h } = domUtils;

const domParser = new DOMParser();

async function transformResults(
  query: Query,
  isFirst: boolean,
  container: HTMLElement,
  sourceHtmlFilesUrl: string,
): Promise<void> {
  const termRegex = new RegExp(
    `(^|\\W)(${query.aggregatedTerms.map((t) => `(${escapeRegex(t)})`).join('|')})(?=\\W|$)`,
    'gi',
  );

  const fragment = document.createDocumentFragment();
  const termInfoEls = isFirst ? displayTermInfo(query) : [];
  termInfoEls.forEach((el) => fragment.appendChild(el));

  let now = performance.now();

  const results = await query.retrieve(10);

  console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);
  now = performance.now();

  const resultsEls = await Promise.all(results.map(async (result) => {
    console.log(result);

    const rawLink = result.getSingleField('link');
    const fullLink = `${sourceHtmlFilesUrl}/${rawLink}`;
    let title = result.getSingleField('title') || rawLink;
    let bodies = transformText(
      result.getStorageWithFieldNames().filter((v) => v[0] !== 'title'),
      query.aggregatedTerms,
      termRegex,
      rawLink,
    );

    if (!bodies.length) {
      const asText = await (await fetch(fullLink)).text();
      const doc = domParser.parseFromString(asText, 'text/html');

      const titles = doc.getElementsByTagName('title');
      if (titles.length) {
        title = titles[0].innerText || title;
      }

      bodies = transformHtml(doc, query.aggregatedTerms, termRegex, rawLink);
    }

    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: fullLink },
        h('div', { class: 'librarian-title' }, title),
        ...bodies));
  }));
  if (resultsEls.length) {
    resultsEls.forEach((el) => fragment.appendChild(el));
  } else if (isFirst) {
    fragment.appendChild(h('div', { class: 'librarian-no-results' }, 'no results found'));
  }
  const sentinel = h('li', {});
  fragment.appendChild(sentinel);

  if (isFirst) {
    container.innerHTML = '';
    container.appendChild(fragment);
  } else {
    container.appendChild(fragment);
  }

  console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  let firstRun = true;
  const iObserver = new IntersectionObserver(async (entries, observer) => {
    if (firstRun || !entries[0].isIntersecting) {
      firstRun = false;
      return;
    }

    observer.unobserve(sentinel);
    sentinel.remove();
    await transformResults(query, false, container, sourceHtmlFilesUrl);
  }, { root: container, rootMargin: '10px 10px 10px 10px' });
  iObserver.observe(sentinel);
}

function displayTermInfo(query: Query): HTMLElement[] {
  const misspelledTerms: string[] = [];
  const correctedTerms: string[] = [];
  const returnVal: HTMLElement[] = [];
  const correctedTermsContainer = h('div', { class: 'librarian-suggestion-container-corrected' },
    h('div', { class: 'librarian-suggestion-buttons' },
      h('button', {
        class: 'librarian-suggestion-button-dismiss',
        onclick: '() => console.log(\'hi\')',
      }),
      h('button', { class: 'librarian-suggestion-button-dismiss-tip' })));

  query.queryParts.forEach((queryPart) => {
    if (queryPart.isCorrected) {
      for (const misspelledTerm of queryPart.originalTerms) {
        if (!queryPart.terms.includes(misspelledTerm)) {
          misspelledTerms.push(misspelledTerm);
        }
      }
      for (const term of queryPart.terms) {
        correctedTerms.push(term);
      }
    } else if (queryPart.isExpanded) {
      returnVal.push(
        h('div', { class: 'librarian-suggestion-container-expanded' },
          h('div', { class: 'librarian-suggestion-content' },
            'Also searched for... ',
            h('small', {}, '(add a space to the last term to finalise the search)'),
            h('br', {}),
            ...queryPart.terms.map((expandedTerm, idx) => (idx === 0 ? '' : h(
              'span', { class: 'librarian-suggestion-expanded' }, `${expandedTerm} `,
            )))),
          h('div', { class: 'librarian-suggestion-buttons' },
            h('button', { class: 'librarian-suggestion-button-dismiss' }),
            h('button', { class: 'librarian-suggestion-button-dismiss-tip' }))),
      );
    }
  });

  if (misspelledTerms.length) {
    correctedTermsContainer.prepend(
      h('div', { class: 'librarian-suggestion-content' },
        'Could not find any matches for',
        ...misspelledTerms.map((term) => h(
          'span', { class: 'librarian-suggestion-wrong' }, ` "${term}"`,
        )),
        correctedTerms.length ? ', searched for: ' : '',
        ...correctedTerms.map((correctedTerm) => h(
          'span', { class: 'librarian-suggestion-corrected' }, `${correctedTerm} `,
        ))),
    );
    returnVal.push(correctedTermsContainer);
  }

  return returnVal;
}

const updatePromiseQueue: (() => Promise<void>)[] = [];
async function update(
  queryString: string,
  container: HTMLElement,
  searcher: Searcher,
  sourceHtmlFilesUrl: string,
): Promise<void> {
  try {
    const now = performance.now();
    const query = await searcher.getQuery(queryString);

    console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    await transformResults(query, true, container, sourceHtmlFilesUrl);

    show(container);
  } catch (ex) {
    container.innerHTML = ex.message;
    throw ex;
  } finally {
    updatePromiseQueue.shift();
    if (updatePromiseQueue.length) {
      await updatePromiseQueue[0]();
    }
  }
}

function hide(container: HTMLElement): void {
  (container.previousSibling as HTMLElement).style.display = 'none';
  container.style.display = 'none';
}

function show(container: HTMLElement): void {
  (container.previousSibling as HTMLElement).style.display = 'block';
  container.style.display = 'block';
}

function initLibrarian(
  librarianOutputUrl: string,
  workerUrl: string,
  sourceHtmlFilesUrl: string,
): void {
  const input = document.getElementById('librarian-search');
  if (!input) {
    return;
  }

  const container = h('ul', { class: 'librarian-dropdown', style: 'display: none;' });
  const parent = input.parentElement;
  input.remove();
  parent.appendChild(h('div', { class: 'librarian-input-wrapper' },
    input,
    h('div', { class: 'librarian-input-dropdown-separator', style: 'display: none;' }),
    container));

  const isMobile = window.matchMedia('only screen and (max-width: 1024px)').matches;

  const searcher = new Searcher({
    url: librarianOutputUrl,
    workerUrl,
    useQueryTermExpansion: !isMobile,
    useQueryTermProximity: !isMobile,
  });

  let inputTimer: any = -1;
  input.addEventListener('input', (ev) => {
    const query = (ev.target as HTMLInputElement).value;

    if (query.length > 2) {
      clearTimeout(inputTimer);
      inputTimer = setTimeout(() => {
        updatePromiseQueue.push(() => update(query, container, searcher, sourceHtmlFilesUrl));
        if (updatePromiseQueue.length === 1) {
          updatePromiseQueue[0]();
        }
      }, 250);
    } else if (query.length < 2) {
      hide(container);
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

initLibrarian(
  'http://192.168.10.132:3000/output',
  '/worker.bundle.js',
  'http://192.168.10.132:3000/source',
);
