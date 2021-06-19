import * as escapeRegex from 'escape-string-regexp';

import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Query from './results/Query';

const { h } = domUtils;

const BODY_SERP_BOUND = 40;
const MAX_SERP_HIGHLIGHT_PARTS = 8;

function transformText(
  texts: [string, string][], // field name - field content pairs
  sortedQueryTerms: string[],
  baseUrl: string,
): (string | HTMLElement)[] {
  const termRegex = new RegExp(
    `(^|\\W)(${sortedQueryTerms.map((t) => escapeRegex(t)).join('|')})(?=\\W|$)`,
    'gi',
  );
  const lowerCasedSortedQueryTerms = sortedQueryTerms.map((t) => t.toLowerCase());

  function getBestMatchResult(str: string): (string | HTMLElement)[] {
    const lastTermPositions = sortedQueryTerms.map(() => -100000000);
    let lastClosestWindowLen = 100000000;
    let lastNumberMatchedTerms = 0;
    let lastClosestWindowPositions: [number, string][] = lastTermPositions.map((i) => [i, '']);

    let match;
    // eslint-disable-next-line no-cond-assign
    while (match = termRegex.exec(str)) {
      const matchedText = match[2].toLowerCase();

      const sortedQueryTermIdx = lowerCasedSortedQueryTerms.findIndex(
        (term) => matchedText.includes(term),
      );
      lastTermPositions[sortedQueryTermIdx] = match.index + match[1].length;

      const filteredPositions = lastTermPositions.filter((p) => p >= 0);
      const windowLen = Math.max(...filteredPositions) - Math.min(...filteredPositions);
      if (filteredPositions.length > lastNumberMatchedTerms || windowLen < lastClosestWindowLen) {
        if (filteredPositions.length > lastNumberMatchedTerms) {
          lastNumberMatchedTerms = filteredPositions.length;
        }
        lastClosestWindowLen = windowLen;
        lastClosestWindowPositions = lastTermPositions.map((i, idx) => [
          i,
          idx === sortedQueryTermIdx ? match[0] : lastClosestWindowPositions[idx][1],
        ]);
      }
    }

    const result: (string | HTMLElement)[] = [];
    lastClosestWindowPositions = lastClosestWindowPositions
      .filter((pair) => pair[0] >= 0)
      .sort((a, b) => a[0] - b[0]);
    if (!lastClosestWindowPositions.length) {
      return result;
    }

    let prevHighlightEndPos = 0;
    for (let i = 0; i < lastClosestWindowPositions.length; i += 1) {
      const pos = lastClosestWindowPositions[i][0];
      const matchedText = lastClosestWindowPositions[i][1];
      const highlightEndPos = pos + matchedText.length;
      if (pos > prevHighlightEndPos + BODY_SERP_BOUND * 2) {
        result.push(' ... ');
        result.push(str.substring(pos - BODY_SERP_BOUND, pos));
        result.push(h('span', { class: 'librarian-highlight' }, matchedText));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      } else {
        result.pop();
        result.push(str.substring(prevHighlightEndPos, pos));
        result.push(h('span', { class: 'librarian-highlight' }, matchedText));
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

async function transformResults(query: Query, container: HTMLElement): Promise<void> {
  const resultsEls = (await query.retrieve(10)).map((result) => {
    console.log(result);

    const link = result.getSingleField('link');
    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: link },
        h('div', { class: 'librarian-title' },
          result.getSingleField('title') || link),
        ...transformText(result.getStorageWithFieldNames().filter((v) => v[0] !== 'title'),
          query.aggregatedTerms, link)));
  });
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
    await transformResults(query, container);
  }, { root: container, rootMargin: '10px 10px 10px 10px' });
  iObserver.observe(sentinel);
}

function displayTermInfo(query: Query, container: HTMLElement) {
  query.queryVectors.forEach((queryVec) => {
    if (Object.keys(queryVec.correctedTermsAndWeights).length) {
      container.appendChild(
        h('div', { class: 'librarian-suggestion-container-corrected' },
          h('div', { class: 'librarian-suggestion-content' },
            `Could not find any matches for "${queryVec.mainTerm}", did you mean:`,
            h('hr', {}),
            ...Object.keys(queryVec.correctedTermsAndWeights).map((correctedTerm) => h(
              'span', { class: 'librarian-suggestion-corrected' }, `${correctedTerm} `,
            ))),
          h('div', { class: 'librarian-suggestion-buttons' },
            h('button', { class: 'librarian-suggestion-button-dismiss', onclick: '() => console.log(\'hi\')' }),
            h('button', { class: 'librarian-suggestion-button-dismiss-tip' }))),
      );
    } else if (Object.keys(queryVec.expandedTermsAndWeights).length) {
      container.appendChild(
        h('div', { class: 'librarian-suggestion-container-expanded' },
          h('div', { class: 'librarian-suggestion-content' },
            'Also searched for...',
            h('br', {}),
            h('small', {}, 'Add a space to the last term to finalise the search!'),
            h('hr', {}),
            ...Object.keys(queryVec.expandedTermsAndWeights).map((expandedTerm) => h(
              'span', { class: 'librarian-suggestion-expanded' }, `${expandedTerm} `,
            ))),
          h('div', { class: 'librarian-suggestion-buttons' },
            h('button', { class: 'librarian-suggestion-button-dismiss' }),
            h('button', { class: 'librarian-suggestion-button-dismiss-tip' }))),
      );
    }
  });
}

const updatePromiseQueue: (() => Promise<void>)[] = [];
async function update(queryString: string, container: HTMLElement, searcher: Searcher): Promise<void> {
  container.style.display = 'flex';

  const query = await searcher.getQuery(queryString);
  container.innerHTML = '';
  displayTermInfo(query, container);

  await transformResults(query, container);

  updatePromiseQueue.shift();
  if (updatePromiseQueue.length) {
    updatePromiseQueue[0]();
  }
}

function hide(container): void {
  container.style.display = 'none';
}

function initLibrarian(url: string, setupDictionaryUrl: string): void {
  const input = document.getElementById('librarian-search');
  if (!input) {
    return;
  }

  const container = h('ul', { class: 'librarian-dropdown' });
  input.parentElement.appendChild(container);

  const searcher = new Searcher(url, setupDictionaryUrl);

  let inputTimer: any = -1;
  input.addEventListener('input', (ev) => {
    console.log('fired');
    const query = (ev.target as HTMLInputElement).value;

    if (query.length > 2) {
      clearTimeout(inputTimer);
      inputTimer = setTimeout(() => {
        updatePromiseQueue.push(() => update(query, container, searcher));
        if (updatePromiseQueue.length === 1) {
          updatePromiseQueue[0]();
        }
      }, 250);
    } else if (query.length < 2) {
      hide(container);
    }
  });
}

initLibrarian('http://localhost:3000', 'http://localhost:8080/setupDictionary.bundle.js');
