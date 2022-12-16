import { Options, UiMode } from '../Options';
import { listItemRender } from '../searchResultTransform/listItemRender';
import { TRANSLATIONS } from '../translations/en';

export function prepareOptions(options: Options) {
  options.searcherOptions = options.searcherOptions || ({} as any);
  
  // ------------------------------------------------------------
  // Ui Options
  
  options.uiOptions = options.uiOptions || ({} as any);
  const { uiOptions } = options;
  
  if (uiOptions.sourceFilesUrl && !uiOptions.sourceFilesUrl.endsWith('/')) {
    uiOptions.sourceFilesUrl += '/';
  }
  
  uiOptions.mode = uiOptions.mode || UiMode.Auto;

  uiOptions.isMobileDevice = uiOptions.isMobileDevice
      || (() => window.matchMedia('only screen and (max-width: 768px)').matches);
  
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
  
  if (!('inputDebounce' in uiOptions)) {
    uiOptions.inputDebounce = 100;
  }
  
  uiOptions.preprocessQuery = uiOptions.preprocessQuery || ((q) => q);
  
  uiOptions.dropdownAlignment = uiOptions.dropdownAlignment || 'bottom-end';
  
  if (typeof uiOptions.fsContainer === 'string') {
    uiOptions.fsContainer = document.getElementById(uiOptions.fsContainer) as HTMLElement;
  }
  uiOptions.fsContainer = uiOptions.fsContainer || document.getElementsByTagName('body')[0] as HTMLElement;
  
  uiOptions.resultsPerPage = uiOptions.resultsPerPage || 10;
  uiOptions.maxSubMatches = uiOptions.maxSubMatches || 2;
  
  uiOptions.translations = {
    ...TRANSLATIONS,
    ...(uiOptions.translations || {}),
  };
  if (!('fsScrollLock' in uiOptions)) {
    uiOptions.fsScrollLock = true;
  }
  uiOptions.sortFields = uiOptions.sortFields || {};
  uiOptions.multiSelectFilters = uiOptions.multiSelectFilters || [];
  uiOptions.numericFilters = uiOptions.numericFilters || [];

  uiOptions.listItemRender = uiOptions.listItemRender || listItemRender;
  
  options.otherOptions = options.otherOptions || {};
}
