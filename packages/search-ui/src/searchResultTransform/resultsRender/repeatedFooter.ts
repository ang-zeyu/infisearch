import { Query } from '@morsels/search-lib';
import h from '@morsels/search-lib/lib/utils/dom';
import { Options } from '../../Options';

export function resultSeparator(
  options: Options,
  numResultsSoFar: number,
  isDoneLoading: boolean,
  loadMore: (nResults: number) => Promise<HTMLElement[] | undefined>,
  focusOption: (el: HTMLElement) => void,
  query: Query,
) {
  const { loadingIndicatorRender, resultsPerPage } = options.uiOptions;
  const footer = h('div', { class: 'morsels-footer', tabindex: '-1' });
  if (!query.resultsTotal) {
    return footer;
  }

  const resultsSoFar = h('div',
    { class: 'morsels-footer-so-far' },
    `${numResultsSoFar} of ${query.resultsTotal}`,
  ).outerHTML;

  const loadMoreButton = h('button', {
    class: 'morsels-load-more',
    tabindex: '-1',
  }, 'Load more results');
  const loadMoreButtonWrapped = h('div', {
    class: 'morsels-load-more-opt',
    role: 'option',
  }, loadMoreButton);

  loadMoreButtonWrapped.addEventListener('focusout', (ev) => {
    // Prevent removing the button from closing the dropdown
    ev.stopPropagation();
  });

  loadMoreButtonWrapped.onclick = () => {
    // Was the button clicked as a result of the combobox controls?
    const isDomFocused = document.activeElement === loadMoreButton;

    loadMoreButtonWrapped.remove();
    footer.append(loadingIndicatorRender(h, options, false, true));
    // Announce footer information
    if (isDomFocused) footer.focus({ preventScroll: true });

    loadMore(resultsPerPage).then((newResultEls) => {
      footer.innerHTML = resultsSoFar;
      footer.classList.add('morsels-footer-loaded');

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

  if (isDoneLoading) {
    footer.innerHTML = resultsSoFar;
  } else {
    footer.append(loadMoreButtonWrapped);
  }

  return footer;
}