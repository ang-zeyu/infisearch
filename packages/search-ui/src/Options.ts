import { SearcherOptions, MorselsConfig } from '@morsels/search-lib/lib/results/Config';
import Result from '@morsels/search-lib/lib/results/Result';
import { Query } from '@morsels/search-lib';
import { CreateElement } from './utils/dom';

export type ArbitraryOptions = any;

export enum UiMode {
  Auto = 'auto',
  Dropdown = 'dropdown',
  Fullscreen = 'fullscreen',
  Target = 'target',
}

export interface UiOptions {
  input: HTMLInputElement,
  inputDebounce?: number,
  preprocessQuery: (input: string) => string,
  mode: UiMode,
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
  maxSubMatches?: number,
  useBreadcrumb?: boolean,
  // This is specific to the default resultsRender implementation,
  // pulling it up as its a common option
  sourceFilesUrl?: string,

  // -----------------------------------------------------
  // Renderers

  // Miscellaneous
  loadingIndicatorRender?: (
    h: CreateElement,
    opts: Options,
    isInitialising: boolean,
    wasResultsBlank: boolean,
  ) => HTMLElement,
  headerRender?: (
    h: CreateElement,
    opts: Options,
    error: boolean,
    blank: boolean,
    queryParts?: Query,
  ) => HTMLElement,

  // Rendering Results
  resultsRender?: (
    h: CreateElement,
    opts: Options,
    config: MorselsConfig,
    results: Result[],
    query: Query,
  ) => Promise<HTMLElement[]>,
  resultsRenderOpts?: {
    addSearchedTerms?: string,
    listItemRender?: (
      h: CreateElement,
      opts: Options,
      searchedTermsJSON: string,
      fullLink: string,
      resultTitle: string,
      resultHeadingsAndTexts: (HTMLElement | string)[],
      fields: [string, string][],
    ) => HTMLElement,
    headingBodyRender?: (
      h: CreateElement,
      opts: Options,
      headingHighlights: (HTMLElement | string)[],
      bodyHighlights: (HTMLElement | string)[],
      href?: string
    ) => HTMLElement,
    bodyOnlyRender?: (
      h: CreateElement,
      opts: Options,
      bodyHighlights: (HTMLElement | string)[],
    ) => HTMLElement,
    highlightRender?: (
      h: CreateElement,
      opts: Options,
      matchedPart: string,
    ) => HTMLElement,
  },
}

export interface Options {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
  isMobileDevice: () => boolean,
  otherOptions: ArbitraryOptions
}
