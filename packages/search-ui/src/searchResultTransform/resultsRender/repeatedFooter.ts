import { Query } from '@infisearch/search-lib';
import h from '@infisearch/search-lib/lib/utils/dom';
import { Options } from '../../Options';
import { stateRender } from '../../utils/state';

export function resultSeparator(
  options: Options,
  numResultsSoFar: number,
  loadMore: (nResults: number) => Promise<HTMLElement[] | undefined>,
  focusOption: (el: HTMLElement) => void,
  query: Query,
) {
  const { resultsPerPage, translations } = options.uiOptions;
  const footer = h('div', { class: 'infi-footer', tabindex: '-1' });
  if (!query.resultsTotal) {
    return footer;
  }

  const resultsSoFar = h('div',
    { class: 'infi-footer-so-far' },
    `${numResultsSoFar} of ${query.resultsTotal}`,
  ).outerHTML;

  const loadMoreButton = h('button', {
    class: 'infi-load-more',
    tabindex: '-1',
    type: 'button',
  }, 'Load more results');
  const loadMoreButtonWrapped = h('div', {
    class: 'infi-load-more-opt',
    role: 'option',
  }, loadMoreButton);

  loadMoreButtonWrapped.addEventListener('focusout', (ev) => {
    // Prevent removing the button from closing the dropdown
    ev.stopPropagation();
  });

  loadMoreButtonWrapped.onclick = (ev) => {
    ev.preventDefault();

    // Was the button clicked as a result of the combobox controls?
    const isDomFocused = document.activeElement === loadMoreButton;

    loadMoreButtonWrapped.remove();
    footer.append(stateRender(false, true, false, false, false, translations));
    // Announce footer information
    if (isDomFocused) footer.focus({ preventScroll: true });

    loadMore(resultsPerPage).then((newResultEls) => {
      footer.innerHTML = resultsSoFar;
      footer.classList.add('infi-footer-loaded');

      if (newResultEls && newResultEls.length && !isDomFocused) {
        const firstEl = newResultEls[0];
        focusOption(
          firstEl.getAttribute('role') === 'option'
            ? firstEl
            : firstEl.querySelector('[role="option"]'),
        );
      }
    });
  };

  if (numResultsSoFar >= query.resultsTotal) {
    footer.innerHTML = resultsSoFar;
  } else {
    footer.append(loadMoreButtonWrapped);
  }

  return footer;
}