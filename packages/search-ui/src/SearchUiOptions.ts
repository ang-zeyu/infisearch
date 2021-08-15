import { SearcherOptions } from '@morsels/search-lib/lib/results/SearcherOptions';
import { MorselsConfig } from '@morsels/search-lib/lib/results/FieldInfo';
import Result from '@morsels/search-lib/lib/results/Result';
import { Query } from '@morsels/search-lib';
import { CreateElement } from './utils/dom';

export interface SearchUiRenderOptions {
  enablePortal?: boolean | 'auto',
  portalTo?: HTMLElement,
  show?: (root: HTMLElement, isPortal: boolean) => void,
  hide?: (root: HTMLElement, isPortal: boolean) => void,
  rootRender?: (
    h: CreateElement,
    inputEl: HTMLElement,
    portalCloseHandler?: () => void,
  ) => ({ root: HTMLElement, listContainer: HTMLElement }),
  portalInputRender?: (h: CreateElement) => HTMLInputElement,
  noResultsRender?: (h: CreateElement) => HTMLElement,
  portalBlankRender?: (h: CreateElement) => HTMLElement,
  loadingIndicatorRender?: (h: CreateElement) => HTMLElement,
  termInfoRender?: (
    h: CreateElement,
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
  listItemRender?: (
    h: CreateElement,
    fullLink: string,
    resultTitle: string,
    resultHeadingsAndTexts: (HTMLElement | string)[],
    fields: [string, string][],
  ) => HTMLElement,
  headingBodyRender?: (
    h: CreateElement,
    heading: string,
    bodyHighlights: (HTMLElement | string)[],
    href?: string
  ) => HTMLElement,
  bodyOnlyRender?: (
    h: CreateElement,
    bodyHighlights: (HTMLElement | string)[],
  ) => HTMLElement,
  highlightRender?: (h: CreateElement, matchedPart: string) => HTMLElement,
}

export interface SearchUiOptions {
  searcherOptions: SearcherOptions,
  inputId: string,
  resultsPerPage?: number,
  sourceFilesUrl?: string,
  render?: SearchUiRenderOptions
}
