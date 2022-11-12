import h from '@infisearch/search-lib/lib/utils/dom';

/**
 * Renders setup, loading, error, idle, blank (input is empty) states
 */
export function stateRender(
  isInitialising: boolean,
  wasResultsBlank: boolean,
  blank: boolean,
  isDone: boolean,
  isError: boolean,
) {
  if (isError) {
    return h('div', { class: 'infi-error' }, 'Oops! Something went wrong... 🙁');
  } else if (blank) {
    return h('div', { class: 'infi-blank' }, 'Start Searching Above!');
  }

  const loadingSpinner = h('span', { class: 'infi-loading-indicator' });
  if (isInitialising) {
    const initialisingText = h('div', { class: 'infi-initialising-text' }, '... Starting Up ...');
    return h('div', {}, loadingSpinner, initialisingText);
  } else if (isDone) {
    return h('div', {});
  }

  if (!wasResultsBlank) {
    loadingSpinner.classList.add('infi-loading-indicator-subsequent');
  }
  
  return loadingSpinner;
}
