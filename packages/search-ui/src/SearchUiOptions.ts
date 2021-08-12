import { SearcherOptions } from '@morsels/search-lib/lib/results/SearcherOptions';

export interface SearchUiOptions {
  searcherOptions: SearcherOptions,
  inputId: string,
  resultsPerPage?: number,
  sourceFilesUrl?: string
}
