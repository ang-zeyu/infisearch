import { SearcherOptions } from '@morsels/search-lib/lib/results/SearcherOptions';
import { MorselsConfig } from '@morsels/search-lib/lib/results/FieldInfo';
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
  fullscreenContainer?: HTMLElement,
  target?: HTMLElement,
  resultsPerPage?: number,
  sourceFilesUrl?: string,

  // -----------------------------------------------------
  // Renderers

  // Dropdown Specific
  dropdownRootRender?: (
    h: CreateElement,
    opts: SearchUiOptions,
    inputEl: HTMLElement,
  ) => ({ dropdownRoot: HTMLElement, dropdownListContainer: HTMLElement }),
  showDropdown?: (
    root: HTMLElement,
    listContainer: HTMLElement,
    opts: SearchUiOptions
  ) => void,
  hideDropdown?: (
    root: HTMLElement,
    listContainer: HTMLElement,
    opts: SearchUiOptions
  ) => void,

  // Fullscreen Specific
  fsRootRender?: (
    h: CreateElement,
    opts: SearchUiOptions,
    fsCloseHandler: () => void,
  ) => ({ root: HTMLElement, listContainer: HTMLElement, input: HTMLInputElement }),
  showFullscreen?: (
    root: HTMLElement,
    listContainer: HTMLElement,
    fullscreenContainer: HTMLElement,
    opts: SearchUiOptions,
  ) => void,
  hideFullscreen?: (
    root: HTMLElement,
    listContainer: HTMLElement,
    fullscreenContainer: HTMLElement,
    opts: SearchUiOptions
  ) => void,

  // Miscellaneous
  noResultsRender?: (h: CreateElement, opts: SearchUiOptions) => HTMLElement,
  fsBlankRender?: (h: CreateElement, opts: SearchUiOptions) => HTMLElement,
  loadingIndicatorRender?: (h: CreateElement, opts: SearchUiOptions) => HTMLElement,
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
    listItemRender?: (
      h: CreateElement,
      opts: SearchUiOptions,
      fullLink: string,
      resultTitle: string,
      resultHeadingsAndTexts: (HTMLElement | string)[],
      fields: [string, string][],
    ) => HTMLElement,
    headingBodyRender?: (
      h: CreateElement,
      opts: SearchUiOptions,
      heading: string,
      bodyHighlights: (HTMLElement | string)[],
      href?: string
    ) => HTMLElement,
    bodyOnlyRender?: (
      h: CreateElement,
      opts: SearchUiOptions,
      bodyHighlights: (HTMLElement | string)[],
    ) => HTMLElement,
    highlightRender?: (h: CreateElement, opts: SearchUiOptions, matchedPart: string) => HTMLElement,
  },
}

export interface SearchUiOptions {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
  isMobileDevice: () => boolean,
  otherOptions: ArbitraryOptions
}
