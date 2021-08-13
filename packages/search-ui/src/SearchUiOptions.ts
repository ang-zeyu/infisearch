import { SearcherOptions } from '@morsels/search-lib/lib/results/SearcherOptions';
import { CreateElement } from './utils/dom';

export interface SearchUiRenderOptions {
  manualPortalControl?: boolean,
  portalTo?: HTMLElement,
  show?: (root: HTMLElement, isPortal: boolean) => void,
  hide?: (root: HTMLElement, isPortal: boolean) => void,
  rootRender?: (
    h: CreateElement, inputEl: HTMLElement, portalCloseHandler?: () => void
  ) => ({ root: HTMLElement, listContainer: HTMLElement }),
  portalInputRender?: (h: CreateElement) => HTMLInputElement,
  loadingIndicatorRender?: (h: CreateElement) => HTMLElement,
  termInfoRender?: (
    h: CreateElement, misspelledTerms: string[], correctedTerms: string[], expandedTerms: string[]
  ) => HTMLElement[],
  listItemRender?: (
    h: CreateElement, fullLink: string, title: string, bodies: (HTMLElement | string)[]
  ) => HTMLElement,
  highlightRender?: (h: CreateElement, matchedPart: string) => HTMLElement,
  headingBodyRender?: (
    h: CreateElement, heading: string, bodyHighlights: (HTMLElement | string)[], href?: string
  ) => HTMLElement,
  bodyOnlyRender?: (h: CreateElement, bodyHighlights: (HTMLElement | string)[]) => HTMLElement,
  noResultsRender?: (h: CreateElement) => HTMLElement,
}

export interface SearchUiOptions {
  searcherOptions: SearcherOptions,
  inputId: string,
  resultsPerPage?: number,
  sourceFilesUrl?: string,
  render?: SearchUiRenderOptions
}
