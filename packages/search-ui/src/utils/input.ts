import { Query } from '@morsels/search-lib';

export class InputState {
  currQuery: Query;

  isRunningNewQuery = false;

  nextQuery: () => any;

  lastElObserver: IntersectionObserver;
}
