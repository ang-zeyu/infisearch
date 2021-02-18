import './styles/search.css';

import Searcher from './results/Searcher';
import domUtils from './utils/dom';
import Results from './results/Results';

const { h } = domUtils;

async function transformResults(results: Results): Promise<HTMLBaseElement[]> {
  return (await results.retrieve(10)).map((result) => {
    const x = 1 + 1;

    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: result.fields.link },
        h('div', { class: 'librarian-heading' }, result.fields.heading)));
  });
}

let isUpdating = false;
async function update(query: string, container: HTMLBaseElement, searcher: Searcher): Promise<void> {
  container.style.display = 'flex';

  const results = await searcher.getResults(query);
  const resultsEls = await transformResults(results);

  container.innerHTML = '';
  resultsEls.forEach((el) => container.appendChild(el));

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
    const query = (ev.target as HTMLInputElement).value;

    if (query.length > 2 && !isUpdating) {
      isUpdating = true;
      update(query, container, searcher);
    } else {
      hide(container);
    }
  });
}

initLibrarian('http://localhost:3000');

export default initLibrarian;
