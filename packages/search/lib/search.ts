import * as escapeRegex from 'escape-string-regexp';

import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Results from './results/Results';

const { h } = domUtils;

const BODY_SERP_BOUND = 40;
const MAX_SERP_HIGHLIGHT_PARTS = 8;

function transformBody(body: string[], query: string): (string | HTMLElement)[] {
  const terms = query.split(/\s+/g);
  const termRegex = new RegExp(terms.map((t) => escapeRegex(t)).join('|'), 'gi');

  function getBoundsForString(str: string): (string | HTMLElement)[] {
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
      }

      const beforeSubstringStart = Math.max(lastInsertedIdxEnd, matchIdx - BODY_SERP_BOUND);
      result.push(` ...${str.substring(beforeSubstringStart, matchIdx)}`);

      result.push(h('mark', {}, matchedText));

      lastInsertedIdxStart = termRegex.lastIndex;
      lastInsertedIdxEnd = Math.min(str.length, lastInsertedIdxStart + BODY_SERP_BOUND);
      result.push(`${str.substring(lastInsertedIdxStart, lastInsertedIdxEnd)}... `);
    }

    return result;
  }

  return body
    .map((origStr) => getBoundsForString(origStr))
    .reduce((acc, next) => acc.concat(next), [])
    .slice(0, MAX_SERP_HIGHLIGHT_PARTS);
}

async function transformResults(results: Results, query: string, container: HTMLElement): Promise<void> {
  const resultsEls = (await results.retrieve(10)).map((result) => {
    console.log(result);

    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: result.fields.link[0] },
        h('div', { class: 'librarian-title' }, result.fields.title[0]),
        h('div', { class: 'librarian-body' }, ...transformBody(result.fields.body, query))));
  });
  resultsEls.forEach((el) => container.appendChild(el));

  const sentinel = h('div', {});
  container.appendChild(sentinel);
  const iObserver = new IntersectionObserver(async (entries, observer) => {
    if (!entries[0].isIntersecting) {
      return;
    }

    observer.unobserve(sentinel);
    await transformResults(results, query, container);
  });
  iObserver.observe(sentinel);
}

let isUpdating = false;
async function update(query: string, container: HTMLElement, searcher: Searcher): Promise<void> {
  container.style.display = 'flex';

  const results = await searcher.getResults(query);
  container.innerHTML = '';

  await transformResults(results, query, container);

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
