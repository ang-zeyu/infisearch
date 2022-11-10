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
    return h('div', { class: 'morsels-error' }, 'Oops! Something went wrong... 🙁');
  } else if (blank) {
    return h('div', { class: 'morsels-blank' }, 'Start Searching Above!');
  }

  const loadingSpinner = h('span', { class: 'morsels-loading-indicator' });
  if (isInitialising) {
    const initialisingText = h('div', { class: 'morsels-initialising-text' }, '... Starting Up ...');
    return h('div', {}, loadingSpinner, initialisingText);
  } else if (isDone) {
    return h('div', {});
  }

  if (!wasResultsBlank) {
    loadingSpinner.classList.add('morsels-loading-indicator-subsequent');
  }
  
  return loadingSpinner;
}
