import { SearcherOptions } from '@morsels/search-lib/lib/results/Config';
import { Result } from '@morsels/search-lib/lib/results/Result';
import { Query } from '@morsels/search-lib';
import { CreateElement } from '@morsels/search-lib/lib/utils/dom';

export type ArbitraryOptions = any;

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
}

export interface UiOptions {
  input: HTMLInputElement,
  inputDebounce?: number,
  preprocessQuery: (input: string) => string,
  mode: UiMode,
  isMobileDevice: () => boolean,
  dropdownAlignment?: 'bottom-start' | 'bottom-end',
  label: string,
  resultsLabel: string,
  fsInputButtonText: string,
  fsInputLabel: string,
  fsContainer?: HTMLElement,
  fsPlaceholder?: string,
  fsCloseText?: string,
  fsScrollLock: boolean,
  target?: HTMLElement,
  tip: boolean,
  resultsPerPage?: number,
  multiSelectFilters: MultiSelectFilterBinding[],
  // This is specific to the default resultsRender implementation,
  // pulling it up as its a common option
  sourceFilesUrl?: string,

  // -----------------------------------------------------
  // Renderers

  // Rendering Results
  listItemRender?: ListItemRender,
  addSearchedTerms?: string,
  useBreadcrumb?: boolean,
  maxSubMatches?: number,
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
  otherOptions: ArbitraryOptions
}
