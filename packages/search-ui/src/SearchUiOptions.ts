import { SearcherOptions, MorselsConfig } from '@morsels/search-lib/lib/results/Config';
import Result from '@morsels/search-lib/lib/results/Result';
import { Query } from '@morsels/search-lib';
import { QueryPart } from '@morsels/search-lib/lib/parser/queryParser';
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
  fsInputLabel: string,
  fsContainer?: HTMLElement,
  fsPlaceholder?: string,
  fsCloseText?: string,
  target?: HTMLElement,
  resultsPerPage?: number,
  // This is specific to the default resultsRender implementation,
  // pulling it up as its a common option
  sourceFilesUrl?: string,

  // -----------------------------------------------------
  // Renderers

  // Miscellaneous
  errorRender?: (h: CreateElement, opts: SearchUiOptions) => HTMLElement,
  noResultsRender?: (h: CreateElement, opts: SearchUiOptions) => HTMLElement,
  fsBlankRender?: (h: CreateElement, opts: SearchUiOptions) => HTMLElement,
  loadingIndicatorRender?: (
    h: CreateElement,
    opts: SearchUiOptions,
    isInitialising: boolean,
    wasResultsBlank: boolean,
  ) => HTMLElement,
  termInfoRender?: (
    h: CreateElement,
    opts: SearchUiOptions,
    queryParts: QueryPart[],
  ) => HTMLElement[],

  // Rendering Results
  resultsRender?: (
    h: CreateElement,
    opts: SearchUiOptions,
    config: MorselsConfig,
    results: Result[],
    query: Query,
  ) => Promise<HTMLElement[]>,
  resultsRenderOpts?: {
    addSearchedTerms?: string,
    listItemRender?: (
      h: CreateElement,
      opts: SearchUiOptions,
      searchedTermsJSON: string,
      fullLink: string,
      resultTitle: string,
      resultHeadingsAndTexts: (HTMLElement | string)[],
      fields: [string, string][],
    ) => HTMLElement,
    headingBodyRender?: (
      h: CreateElement,
      opts: SearchUiOptions,
      headingHighlights: (HTMLElement | string)[],
      bodyHighlights: (HTMLElement | string)[],
      href?: string
    ) => HTMLElement,
    bodyOnlyRender?: (
      h: CreateElement,
      opts: SearchUiOptions,
      bodyHighlights: (HTMLElement | string)[],
    ) => HTMLElement,
    highlightRender?: (
      h: CreateElement,
      opts: SearchUiOptions,
      matchedPart: string,
    ) => HTMLElement,
  },
}

export interface SearchUiOptions {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
  isMobileDevice: () => boolean,
  otherOptions: ArbitraryOptions
}
