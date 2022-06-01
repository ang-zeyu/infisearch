import { Query } from '@morsels/search-lib';
import { createInvisibleLoadingIndicator } from './dom';

export class InputState {
  currQuery: Query;

  /**
   * Are there any results in the list container currently?
   */
  _mrlIsResultsBlank = true;

  _mrlIsRunningQuery = false;

  /**
   * An input will only take one action at a time, and queue only one.
   * This facilitates query pre-emption.
   */
  _mrlNextAction: () => any;

  _mrlLoader: HTMLElement = createInvisibleLoadingIndicator();

  _mrlLastElObserver: IntersectionObserver;
}
