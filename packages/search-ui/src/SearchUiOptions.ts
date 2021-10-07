import { SearcherOptions } from '@morsels/search-lib/lib/results/SearcherOptions';
import { MorselsConfig } from '@morsels/search-lib/lib/results/FieldInfo';
import Result from '@morsels/search-lib/lib/results/Result';
import { Query } from '@morsels/search-lib';
import { CreateElement } from './utils/dom';

export interface ArbitraryRenderOptions {
  [key: string]: any,
  dropdownAlignment?: 'left' | 'right',
}

export interface SearchUiRenderOptions {
  enablePortal?: boolean | 'auto',
  portalTo?: HTMLElement,
  resultsPerPage?: number,
  show?: (
    root: HTMLElement,
    opts: ArbitraryRenderOptions,
    isPortal: boolean
  ) => void,
  hide?: (
    root: HTMLElement,
    opts: ArbitraryRenderOptions,
    isPortal: boolean
  ) => void,
  rootRender?: (
    h: CreateElement,
    opts: ArbitraryRenderOptions,
    inputEl: HTMLElement,
  ) => ({ root: HTMLElement, listContainer: HTMLElement }),
  portalRootRender?: (
    h: CreateElement,
    opts: ArbitraryRenderOptions,
    portalCloseHandler: () => void,
  ) => ({ root: HTMLElement, listContainer: HTMLElement, input: HTMLInputElement }),
  noResultsRender?: (h: CreateElement, opts: ArbitraryRenderOptions) => HTMLElement,
  portalBlankRender?: (h: CreateElement, opts: ArbitraryRenderOptions) => HTMLElement,
  loadingIndicatorRender?: (h: CreateElement, opts: ArbitraryRenderOptions) => HTMLElement,
  termInfoRender?: (
    h: CreateElement,
    opts: ArbitraryRenderOptions,
    misspelledTerms: string[],
    correctedTerms: string[],
    expandedTerms: string[],
  ) => HTMLElement[],
  resultsRender?: (
    h: CreateElement,
    options: SearchUiOptions,
    config: MorselsConfig,
    results: Result[],
    query: Query,
  ) => Promise<HTMLElement[]>,
  resultsRenderOpts?: {
    listItemRender?: (
      h: CreateElement,
      opts: ArbitraryRenderOptions,
      fullLink: string,
      resultTitle: string,
      resultHeadingsAndTexts: (HTMLElement | string)[],
      fields: [string, string][],
    ) => HTMLElement,
    headingBodyRender?: (
      h: CreateElement,
      opts: ArbitraryRenderOptions,
      heading: string,
      bodyHighlights: (HTMLElement | string)[],
      href?: string
    ) => HTMLElement,
    bodyOnlyRender?: (
      h: CreateElement,
      opts: ArbitraryRenderOptions,
      bodyHighlights: (HTMLElement | string)[],
    ) => HTMLElement,
    highlightRender?: (h: CreateElement, opts: ArbitraryRenderOptions, matchedPart: string) => HTMLElement,
  },
  opts?: ArbitraryRenderOptions,
}

export interface SearchUiOptions {
  searcherOptions: SearcherOptions,
  inputId: string,
  inputDebounce?: number,
  sourceFilesUrl?: string,
  render?: SearchUiRenderOptions
}
