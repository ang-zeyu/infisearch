import * as escapeRegex from 'escape-string-regexp';

import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Query from './results/Query';

const { h } = domUtils;

const BODY_SERP_BOUND = 40;
const MAX_SERP_HIGHLIGHT_PARTS = 2;

interface MatchResult {
  result: (string | HTMLElement)[],
  numberTermsMatched: number,
}

interface FinalMatchResult {
  result: string | HTMLElement,
  numberTermsMatched: number,
}

function transformText(
  texts: [string, string][], // field name - field content pairs
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
): (string | HTMLElement)[] {
  const lowerCasedSortedQueryTerms = sortedQueryTerms.map((t) => t.toLowerCase());

  function getBestMatchResult(str: string): MatchResult {
    const lastTermPositions = sortedQueryTerms.map(() => -100000000);
    let lastClosestTermPositions = lastTermPositions.map((i) => i);
    let lastClosestWindowLen = 100000000;
    let lastNumberMatchedTerms = 0;

    let match = termRegex.exec(str);
    while (match) {
      const matchedText = match[2].toLowerCase();

      const matchedQueryTermIdx = lowerCasedSortedQueryTerms.findIndex(
        (term) => matchedText.includes(term),
      );
      lastTermPositions[matchedQueryTermIdx] = match.index + match[1].length;

      const validLastTermPositions = lastTermPositions.filter((p) => p >= 0);
      const windowLen = Math.max(...validLastTermPositions) - Math.min(...validLastTermPositions);

      const isMoreTermsMatched = validLastTermPositions.length > lastNumberMatchedTerms;
      if (isMoreTermsMatched || windowLen < lastClosestWindowLen) {
        if (isMoreTermsMatched) {
          lastNumberMatchedTerms = validLastTermPositions.length;
        }
        lastClosestWindowLen = windowLen;

        lastClosestTermPositions = lastTermPositions.map((pos) => pos);
      }

      match = termRegex.exec(str);
    }
    termRegex.lastIndex = 0;

    const lastClosestWindowPositions = lastClosestTermPositions
      .map((pos, idx) => ({ pos, term: sortedQueryTerms[idx] }))
      .filter((pair) => pair.pos >= 0)
      .sort((a, b) => a.pos - b.pos);
    const result: (string | HTMLElement)[] = [];
    if (!lastClosestWindowPositions.length) {
      return { result, numberTermsMatched: lastNumberMatchedTerms };
    }

    let prevHighlightEndPos = 0;
    for (let i = 0; i < lastClosestWindowPositions.length; i += 1) {
      const { pos, term } = lastClosestWindowPositions[i];
      const highlightEndPos = pos + term.length;
      if (pos > prevHighlightEndPos + BODY_SERP_BOUND * 2) {
        result.push(' ... ');
        result.push(str.substring(pos - BODY_SERP_BOUND, pos));
        result.push(h('span', { class: 'librarian-highlight' }, term));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      } else {
        result.pop();
        result.push(str.substring(prevHighlightEndPos, pos));
        result.push(h('span', { class: 'librarian-highlight' }, term));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      }
      prevHighlightEndPos = highlightEndPos;
    }
    result.push(' ...');

    return { result, numberTermsMatched: lastNumberMatchedTerms };
  }

  let lastIncludedHeading = -1;
  const finalMatchResults: FinalMatchResult[] = [];

  let itemIdx = -1;
  for (const item of texts) {
    itemIdx += 1;
    if (item[0].startsWith('heading')) {
      continue;
    }

    const { result, numberTermsMatched } = getBestMatchResult(item[1]);
    if (numberTermsMatched === 0) {
      continue;
    }

    const finalMatchResult: FinalMatchResult = { result: undefined, numberTermsMatched };
    finalMatchResults.push(finalMatchResult);

    // Find a new heading this text is under
    let i = itemIdx - 1;
    for (; i > lastIncludedHeading; i -= 1) {
      if (texts[i][0] === 'heading') {
        lastIncludedHeading = i;
        const href = (i - 1 >= 0) && texts[i - 1][0] === 'headingLink'
          ? `${baseUrl}${texts[i - 1][1]}`
          : undefined;
        finalMatchResult.result = h('a', { class: 'librarian-heading-body', href },
          h('div', { class: 'librarian-heading' }, texts[i][1]),
          h('div', { class: 'librarian-bodies' },
            h('div', { class: 'librarian-body' }, ...result)));
        break;
      }
    }

    // Insert without heading
    if (lastIncludedHeading !== i) {
      finalMatchResult.result = h('div', { class: 'librarian-body' }, ...result);
    }
  }

  return finalMatchResults
    .sort((a, b) => b.numberTermsMatched - a.numberTermsMatched)
    .map((r) => r.result)
    .slice(0, MAX_SERP_HIGHLIGHT_PARTS);
}

function transformHtml(
  doc: Document,
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
): (string | HTMLElement)[] {
  const fields: [string, string][] = [];

  function traverseBody(el: HTMLElement) {
    switch (el.tagName.toLowerCase()) {
      case 'h1':
      case 'h2':
      case 'h3':
      case 'h4':
      case 'h5':
      case 'h6':
        fields.push(['heading', el.innerText]);
        break;
      default: {
        for (let i = 0; i < el.childNodes.length; i += 1) {
          const child = el.childNodes[i];
          if (child.nodeType === Node.ELEMENT_NODE) {
            traverseBody(child as HTMLElement);
          } else if (child.nodeType === Node.TEXT_NODE) {
            if (fields.length && fields[fields.length - 1][0] === 'body') {
              fields[fields.length - 1][1] += (child as Text).data;
            } else {
              fields.push(['body', (child as Text).data]);
            }
          }
        }
      }
    }
  }

  const body = doc.getElementsByTagName('body');
  if (body.length) {
    traverseBody(body[0]);
  }

  return transformText(fields, sortedQueryTerms, termRegex, baseUrl);
}

const domParser = new DOMParser();

async function transformResults(
  query: Query,
  isFirst: boolean,
  container: HTMLElement,
  baseUrl: string,
): Promise<void> {
  const termRegex = new RegExp(
    `(^|\\W)(${query.aggregatedTerms.map((t) => escapeRegex(t)).join('|')})(?=\\W|$)`,
    'gi',
  );

  const termInfoEls = isFirst ? displayTermInfo(query) : [];

  const now = performance.now();

  const results = await query.retrieve(10);

  console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);

  const resultsEls = await Promise.all(results.map(async (result) => {
    console.log(result);

    const link = result.getSingleField('link');
    let title = result.getSingleField('title') || link;
    let bodies = transformText(
      result.getStorageWithFieldNames().filter((v) => v[0] !== 'title'),
      query.aggregatedTerms,
      termRegex,
      link,
    );

    if (!bodies.length) {
      const asText = await (await fetch(`${baseUrl}/${link}`)).text();
      const doc = domParser.parseFromString(asText, 'text/html');

      const titles = doc.getElementsByTagName('title');
      if (titles.length) {
        title = titles[0].innerText || title;
      }

      bodies = transformHtml(doc, query.aggregatedTerms, termRegex, link);
    }

    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: link },
        h('div', { class: 'librarian-title' }, title),
        ...bodies));
  }));
  const fragment = document.createDocumentFragment();
  if (resultsEls.length) {
    resultsEls.forEach((el) => fragment.appendChild(el));
  } else {
    fragment.appendChild(h('div', { class: 'librarian-no-results' }, 'no results found'));
  }
  termInfoEls.forEach((el) => fragment.appendChild(el));
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
    await transformResults(query, false, container, baseUrl);
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
      misspelledTerms.push(queryPart.originalTerms[0]);
      if (queryPart.terms.length) {
        correctedTerms.push(queryPart.terms[0]);
      }
    } else if (queryPart.isExpanded) {
      returnVal.push(
        h('div', { class: 'librarian-suggestion-container-expanded' },
          h('div', { class: 'librarian-suggestion-content' },
            'Also searched for... ',
            h('small', {}, '(add a space to the last term to finalise the search)'),
            h('br', {}),
            ...queryPart.terms.map((expandedTerm) => h(
              'span', { class: 'librarian-suggestion-expanded' }, `${expandedTerm} `,
            ))),
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
  sourceHtmlFilesUrl?: string,
): Promise<void> {
  try {
    if (container.style.display === 'none') {
      (container.previousSibling as HTMLElement).style.display = 'block';
      container.style.display = 'block';
    }

    const now = performance.now();
    const query = await searcher.getQuery(queryString);

    console.log(`getQuery "${queryString}" took ${performance.now() - now} milliseconds`);

    await transformResults(query, true, container, sourceHtmlFilesUrl);
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

function initLibrarian(
  librarianOutputUrl: string,
  setupDictionaryUrl: string,
  sourceHtmlFilesUrl?: string,
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

  const searcher = new Searcher(librarianOutputUrl, setupDictionaryUrl);

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

  input.addEventListener('blur', () => hide(container));
}

initLibrarian(
  'http://192.168.10.132:3000/output',
  '/setupDictionary.bundle.js',
  'http://192.168.10.132:3000/source',
);
