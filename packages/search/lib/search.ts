import * as escapeRegex from 'escape-string-regexp';

import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Query from './results/Query';

const { h } = domUtils;

const BODY_SERP_BOUND = 40;
const MAX_SERP_HIGHLIGHT_PARTS = 3;

function transformText(
  texts: [string, string][], // field name - field content pairs
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
): (string | HTMLElement)[] {
  const lowerCasedSortedQueryTerms = sortedQueryTerms.map((t) => t.toLowerCase());

  function getBestMatchResult(str: string): (string | HTMLElement)[] {
    const lastTermPositions = sortedQueryTerms.map(() => -100000000);
    const lastClosestTermPositions = lastTermPositions.map((i) => i);
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

        lastClosestTermPositions[matchedQueryTermIdx] = lastTermPositions[matchedQueryTermIdx];
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
      return result;
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

    return result;
  }

  let lastIncludedHeading = -1;
  const result: (string |HTMLElement)[] = [];

  texts.forEach((item, idx) => {
    if (item[0].startsWith('heading')) {
      return;
    }

    const bodyMatchResult = getBestMatchResult(item[1]);
    if (bodyMatchResult.length === 0) {
      return;
    }

    // Find a new heading this text is under
    for (let i = idx - 1; i > lastIncludedHeading; i -= 1) {
      if (texts[i][0] === 'heading') {
        lastIncludedHeading = i;
        const href = (i - 1 >= 0) && texts[i - 1][0] === 'headingLink'
          ? `${baseUrl}${texts[i - 1][1]}`
          : undefined;
        result.push(h('a', { class: 'librarian-heading-body', href },
          h('div', { class: 'librarian-heading' }, texts[i][1]),
          h('div', { class: 'librarian-bodies' },
            h('div', { class: 'librarian-body' }, ...bodyMatchResult))));
        return;
      }
    }

    const lastResultAdded = result[result.length - 1] as HTMLElement;
    if (lastResultAdded && lastResultAdded.classList.contains('librarian-heading-body')) {
      const headingBodyContainer = lastResultAdded.children[1];
      if (headingBodyContainer.childElementCount < 3) {
        // Insert under previous heading
        headingBodyContainer.appendChild(h('div', { class: 'librarian-body' }, ...bodyMatchResult));
      }
    } else {
      // Insert without heading
      result.push(h('div', { class: 'librarian-body' }, ...bodyMatchResult));
    }
  });

  return result.slice(0, MAX_SERP_HIGHLIGHT_PARTS);
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

async function transformResults(query: Query, container: HTMLElement, baseUrl: string): Promise<void> {
  const termRegex = new RegExp(
    `(^|\\W)(${query.aggregatedTerms.map((t) => escapeRegex(t)).join('|')})(?=\\W|$)`,
    'gi',
  );

  const resultsEls = await Promise.all((await query.retrieve(10)).map(async (result) => {
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
  resultsEls.forEach((el) => container.appendChild(el));

  const sentinel = h('li', {});
  container.appendChild(sentinel);

  let firstRun = true;
  const iObserver = new IntersectionObserver(async (entries, observer) => {
    if (firstRun || !entries[0].isIntersecting) {
      firstRun = false;
      return;
    }

    observer.unobserve(sentinel);
    sentinel.remove();
    await transformResults(query, container, baseUrl);
  }, { root: container, rootMargin: '10px 10px 10px 10px' });
  iObserver.observe(sentinel);
}

function displayTermInfo(query: Query, container: HTMLElement) {
  const misspelledTerms: string[] = [];
  const correctedTerms: string[] = [];
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
      container.appendChild(
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
    container.appendChild(correctedTermsContainer);
  }
}

const updatePromiseQueue: (() => Promise<void>)[] = [];
async function update(
  queryString: string,
  container: HTMLElement,
  searcher: Searcher,
  sourceHtmlFilesUrl?: string,
): Promise<void> {
  try {
    container.style.display = 'flex';

    const now = performance.now();
    const query = await searcher.getQuery(queryString);
    container.innerHTML = '';
    displayTermInfo(query, container);

    await transformResults(query, container, sourceHtmlFilesUrl);

    console.log(`Query "${queryString}" took ${performance.now() - now} milliseconds`);
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

function hide(container): void {
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

  const container = h('ul', { class: 'librarian-dropdown' });
  input.parentElement.appendChild(container);

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
}

initLibrarian(
  'http://localhost:3000/output',
  'http://localhost:8080/setupDictionary.bundle.js',
  'http://localhost:3000/source',
);
