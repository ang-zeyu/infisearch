import { Query } from '@morsels/search-lib';

export class InputState {
  currQuery: Query;

  /**
   * Are there any results in the list container currently?
   */
  isResultsBlank = true;

  isRunningQuery = false;

  /**
   * An input will only take one action at a time, and queue only one.
   * This facilitates query pre-emption.
   */
  nextAction: () => any;

  lastElObserver: IntersectionObserver;
}
