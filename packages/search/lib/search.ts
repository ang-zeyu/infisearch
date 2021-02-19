import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Results from './results/Results';

const { h } = domUtils;

const BODY_SERP_BOUND = 40;

function transformBody(body: string[], query: string): string {
  const terms = query.split(/\s+/g);

  function getBoundsForString(originalStr: string, lowerCasedStr: string) {
    const bounds: number[][] = [];
    terms.forEach((term) => {
      const termIdx = lowerCasedStr.indexOf(term);
      if (termIdx === -1) {
        return;
      }

      const minBound = Math.max(0, termIdx - BODY_SERP_BOUND);
      const maxBound = Math.min(body.length, termIdx + BODY_SERP_BOUND);

      let mergedBound = false;
      for (let i = 0; i < bounds.length; i += 1) {
        if ((minBound <= bounds[i][1] && minBound >= bounds[i][0])
          || (maxBound >= bounds[i][0] && maxBound <= bounds[i][1])) {
          mergedBound = true;

          bounds[i][0] = Math.min(minBound, bounds[i][0]);
          bounds[i][1] = Math.max(maxBound, bounds[i][1]);
          break;
        }
      }

      if (!mergedBound) {
        bounds.push([minBound, maxBound]);
      }
    });

    return bounds
      .map((bound) => `... ${originalStr.substring(bound[0], bound[1])} ...`)
      .reduce((x, y) => `${x} ${y}`, '');
  }

  const lowerCasedBody = body.map((str) => str.toLowerCase());

  return body
    .map((origStr, idx) => getBoundsForString(origStr, lowerCasedBody[idx]))
    .reduce((x, y) => `${x} ${y}`);
}

async function transformResults(results: Results, query: string, container: HTMLBaseElement): Promise<void> {
  const resultsEls = (await results.retrieve(10)).map((result) => {
    console.log(result);

    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: result.fields.link[0] },
        h('div', { class: 'librarian-heading' }, result.fields.title[0]),
        h('div', { class: 'librarian-body' }, transformBody(result.fields.body, query))));
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
async function update(query: string, container: HTMLBaseElement, searcher: Searcher): Promise<void> {
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
