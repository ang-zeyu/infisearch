import * as escapeRegex from 'escape-string-regexp';

import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Query from './results/Query';

const { h } = domUtils;

const BODY_SERP_BOUND = 40;
const MAX_SERP_HIGHLIGHT_PARTS = 8;

function transformText(
  texts: { fieldName: string, text: string }[],
  queriedTerms: string[],
  baseUrl: string,
): (string | HTMLElement)[] {
  const termRegex = new RegExp(queriedTerms.map((t) => escapeRegex(t)).join('|'), 'gi');

  function getMatchResult(str: string): (string | HTMLElement)[] {
    const result = [];

    let lastInsertedIdxStart = 0;
    let lastInsertedIdxEnd = 0;
    let match;
    // eslint-disable-next-line no-cond-assign
    while (match = termRegex.exec(str)) {
      const matchedText = match[0];
      const matchIdx = match.index;

      if (lastInsertedIdxEnd > matchIdx) {
        result.pop();
        lastInsertedIdxEnd = lastInsertedIdxStart;
      } else if (lastInsertedIdxEnd > 0) {
        result.push(' ...');
      }

      const beforeSubstringStart = Math.max(lastInsertedIdxEnd, matchIdx - BODY_SERP_BOUND);
      result.push(str.substring(beforeSubstringStart, matchIdx));

      result.push(h('span', { class: 'librarian-highlight' }, matchedText));

      lastInsertedIdxStart = termRegex.lastIndex;
      lastInsertedIdxEnd = Math.min(str.length, lastInsertedIdxStart + BODY_SERP_BOUND);
      result.push(`${str.substring(lastInsertedIdxStart, lastInsertedIdxEnd)} ... `);
    }

    return result;
  }

  let lastIncludedHeading = -1;
  const result: (string |HTMLElement)[] = [];

  texts.forEach((item, idx) => {
    if (item.fieldName.startsWith('heading')) {
      return;
    }

    const bodyMatchResult = getMatchResult(item.text);
    if (bodyMatchResult.length === 0) {
      return;
    }

    // Find a new heading this text is under
    for (let i = idx - 1; i > lastIncludedHeading; i -= 1) {
      if (texts[i].fieldName === 'heading') {
        lastIncludedHeading = i;
        const href = texts[i - 1]?.fieldName === 'headingLink'
          ? `${baseUrl}${texts[i - 1].text}`
          : undefined;
        result.push(h('a', { class: 'librarian-heading-body', href },
          h('div', { class: 'librarian-heading' }, texts[i].text),
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

async function transformResults(results: Query, container: HTMLElement): Promise<void> {
  const resultsEls = (await results.retrieve(10)).map((result) => {
    console.log(result);

    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: result.storages.link },
        h('div', { class: 'librarian-title' }, result.storages.title),
        ...transformText(result.storages.text, results.queriedTerms, result.storages.link)));
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
    await transformResults(results, container);
  }, { root: container, rootMargin: '10px 10px 10px 10px' });
  iObserver.observe(sentinel);
}

let isUpdating = false;
async function update(query: string, container: HTMLElement, searcher: Searcher): Promise<void> {
  container.style.display = 'flex';

  const results = await searcher.getQuery(query);
  container.innerHTML = '';

  await transformResults(results, container);

  isUpdating = false;
}

function hide(container): void {
  container.style.display = 'none';
}

function initLibrarian(url): void {
  const input = document.getElementById('librarian-search');
  if (!input) {
    return;
  }

  const container = h('ul', { class: 'librarian-dropdown' });
  input.parentElement.appendChild(container);

  const searcher = new Searcher(url);

  input.addEventListener('input', (ev) => {
    const query = (ev.target as HTMLInputElement).value.toLowerCase();

    if (query.length > 2 && !isUpdating) {
      isUpdating = true;
      update(query, container, searcher);
    } else if (query.length < 2) {
      hide(container);
    }
  });
}

initLibrarian('http://localhost:3000');

export default initLibrarian;
