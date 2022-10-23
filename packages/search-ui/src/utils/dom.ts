import h from '@morsels/search-lib/lib/utils/dom';

export const LOADING_INDICATOR_ID = 'data-morsels-loading-indicator';

export function createInvisibleLoadingIndicator(): HTMLElement {
  return h('div', { [LOADING_INDICATOR_ID]: 'true' });
}
