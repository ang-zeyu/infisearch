import { Options, UiMode, UiOptions } from '../Options';
import { listItemRender } from '../searchResultTransform/listItemRender';
import { TRANSLATIONS } from '../translations/en';

export function prepareOptions(options: Options) {
  options.searcherOptions = options.searcherOptions || ({} as any);
  
  // ------------------------------------------------------------
  // Ui Options

  const suppliedUiOpts = (options.uiOptions || {}) as UiOptions;
  options.uiOptions = {
    mode: UiMode.Auto,
    inputDebounce: 100,
    isMobileDevice: () => window.matchMedia('only screen and (max-width: 768px)').matches,
    preprocessQuery: (q) => q,
    dropdownAlignment: 'bottom-end',
    resultsPerPage: 10,
    maxSubMatches: 2,
    fsScrollLock: true,
    fsContainer: document.getElementsByTagName('body')[0] as HTMLElement,
    sortFields: {},
    multiSelectFilters: [],
    numericFilters: [],
    listItemRender,
    ...suppliedUiOpts,
    translations: {
      ...TRANSLATIONS,
      ...(suppliedUiOpts.translations || {}),
    },
  };

  const { uiOptions } = options;
  if (uiOptions.sourceFilesUrl && !uiOptions.sourceFilesUrl.endsWith('/')) {
    uiOptions.sourceFilesUrl += '/';
  }

  if (uiOptions.mode === UiMode.Target) {
    if (typeof uiOptions.target === 'string') {
      uiOptions.target = document.getElementById(uiOptions.target);
    }

    if (!uiOptions.target) {
      throw new Error('\'target\' mode specified but no valid target option specified');
    }
  }

  if (!('input' in uiOptions) || typeof uiOptions.input === 'string') {
    uiOptions.input = document.getElementById(uiOptions.input as any || 'infi-search') as HTMLInputElement;
  }

  if ([UiMode.Dropdown, UiMode.Target].includes(uiOptions.mode) && !uiOptions.input) {
    throw new Error('\'dropdown\' or \'target\' mode specified but no input element found');
  }

  if (typeof uiOptions.fsContainer === 'string') {
    uiOptions.fsContainer = document.getElementById(uiOptions.fsContainer) as HTMLElement;
  }
}
