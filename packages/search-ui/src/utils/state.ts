import h from '@infisearch/search-lib/lib/utils/dom';
import { Translations } from '../Options';

/**
 * Renders setup, loading, error, idle, blank (input is empty) states
 */
export function stateRender(
  isInitialising: boolean,
  wasResultsBlank: boolean,
  blank: boolean,
  isDone: boolean,
  isError: boolean,
  translations: Translations,
) {
  if (isError) {
    return h('div', { class: 'infi-error' }, translations.error);
  } else if (blank) {
    return h('div', { class: 'infi-blank' }, translations.startSearching);
  }

  const loadingSpinner = h('span', { class: 'infi-loading-indicator' });
  if (isInitialising) {
    const initialisingText = h('div', { class: 'infi-initialising-text' }, translations.startingUp);
    return h('div', {}, loadingSpinner, initialisingText);
  } else if (isDone) {
    return h('div', {});
  }

  if (!wasResultsBlank) {
    loadingSpinner.classList.add('infi-loading-indicator-subsequent');
  }
  
  return loadingSpinner;
}
