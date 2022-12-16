import { Query } from '@infisearch/search-lib';
import h from '@infisearch/search-lib/lib/utils/dom';
import { Translations } from '../Options';

function getArrow(invert: boolean) {
  // https://www.svgrepo.com/svg/49189/up-arrow (CC0 License)
  return '<svg class="infi-key-arrow'
          + (invert ? ' infi-key-arrow-down' : '')
          // eslint-disable-next-line max-len
          + '"x="0" y="0" viewBox="0 0 490 490" style="enable-background:new 0 0 490 490" xml:space="preserve"><polygon points="8.081,242.227 82.05,314.593 199.145,194.882 199.145,490 306.14,490 306.14,210.504 407.949,314.593 481.919,242.227 245.004,0"/></svg>';
}

export function headerRender(
  query: Query,
  getOrSetFiltersShown: (setValue?: boolean) => boolean,
  translations: Translations,
) {
  const header = h('div', { class: 'infi-header' });

  if (query) {
    header.append(h('div',
      { class: 'infi-results-found' },
      query.resultsTotal + translations.numResultsFound,
    ));
  }
  
  const instructions = h('div', { class: 'infi-instructions' });
  instructions.innerHTML = translations.navigation
        + getArrow(false)
        + getArrow(true)
        // https://www.svgrepo.com/svg/355201/return (Apache license)
        // eslint-disable-next-line max-len
        + '<svg class="infi-key-return" viewBox="0 0 24 24"><path fill="none" stroke-width="4" d="M9,4 L4,9 L9,14 M18,19 L18,9 L5,9" transform="matrix(1 0 0 -1 0 23)"/></svg>';

  header.append(instructions);

  if (getOrSetFiltersShown) {
    const filters = h('button',
      {
        class: 'infi-filters' + (getOrSetFiltersShown() ? ' active' : ''),
        type: 'button',
      },
      translations.filtersButton,
    );
    filters.onclick = (ev) => {
      ev.preventDefault();
      const shown = getOrSetFiltersShown(!getOrSetFiltersShown());
      if (shown)
        filters.classList.add('active');
      else
        filters.classList.remove('active');
    };
    header.insertBefore(filters, instructions);
  }

  return header;
}