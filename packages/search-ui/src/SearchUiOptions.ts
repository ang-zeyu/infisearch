import { SearcherOptions } from '@morsels/search-lib/lib/results/SearcherOptions';

export interface SearchUiOptions {
  searcherOptions: SearcherOptions,
  resultsPerPage?: number,
  sourceFilesUrl?: string
}
