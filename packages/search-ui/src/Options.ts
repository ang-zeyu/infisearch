import { SearcherOptions } from '@infisearch/search-lib/lib/results/Config';
import { Result } from '@infisearch/search-lib/lib/results/Result';
import { Query } from '@infisearch/search-lib';
import { CreateElement } from '@infisearch/search-lib/lib/utils/dom';

export enum UiMode {
  Auto = 'auto',
  Dropdown = 'dropdown',
  Fullscreen = 'fullscreen',
  Target = 'target',
}

export interface MultiSelectFilterBinding {
  fieldName: string,
  displayName: string,
  defaultOptName: string,
  collapsed?: boolean,
}

export interface NumericFilterBinding {
  fieldName: string,
  displayName: string,
  type: 'number' | 'datetime-local' | 'date',
  gte?: number,
  lte?: number,
  gtePlaceholder?: string,
  ltePlaceholder?: string,
}

export interface Translations {
  resultsLabel: string,
  fsButtonPlaceholder?: string,
  fsButtonLabel: string,
  fsPlaceholder: string,
  fsCloseText: string,
  filtersButton: string,
  numResultsFound: string,
  startSearching: string,
  startingUp: string,
  navigation: string,
  tipHeader: string,
  tip: string,
  example: string,
  tipRows: {
    searchPhrases: string,
    requireTerm: string,
    excludeTerm: string,
    flipResults: string,
    groupTerms: string,
    searchPrefixes: string,
    searchSections: string,
    exSearchPhrases: string,
    exRequireTerm: string,
    exExcludeTerm: string,
    exFlipResults: string,
    exGroupTerms: string,
    exSearchPrefixes: string,
    exSearchSections: string[],
  }

  error: string,
}

export interface UiOptions {
  sourceFilesUrl?: string,
  input: HTMLInputElement,
  inputDebounce?: number,
  preprocessQuery: (input: string) => string,
  mode: UiMode,
  isMobileDevice: () => boolean,
  dropdownAlignment?: 'bottom-start' | 'bottom-end',
  fsContainer?: HTMLElement,
  fsScrollLock: boolean,
  target?: HTMLElement,
  tip: boolean,
  resultsPerPage?: number,
  sortFields: { [fieldName: string]: { asc: string, desc: string } },
  multiSelectFilters: MultiSelectFilterBinding[],
  numericFilters: NumericFilterBinding[],
  translations: Translations,

  // -----------------------------------------------------
  // Rendering Results
  listItemRender?: ListItemRender,
  onLinkClick: (ev: MouseEvent) => void,
  searchedTermsParam?: string,
  useBreadcrumb?: boolean,
  maxSubMatches?: number,
  contentFields: string[],
}

export type ListItemRender = (
  h: CreateElement,
  opts: Options,
  result: Result,
  query: Query,
) => Promise<HTMLElement>;

export interface Options {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
}
