import { Query } from '@morsels/search-lib';

export class InputState {
  currQuery: Query;

  wasResultsBlank = true;

  isRunningQuery = false;

  nextAction: () => any;

  lastElObserver: IntersectionObserver;
}
