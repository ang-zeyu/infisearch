import Result from './Result';
import { QueryPart } from '../parser/queryParser';

class Query {
  constructor(
    /**
     * Original query string.
     */
    public readonly query: string,
    /**
     * Total number of results.
     */
    public readonly resultsTotal: number,
    /**
     * Syntactic tree of query parsed by Morsels.
     */
    public readonly queryParts: QueryPart[],
    /**
     * Returns the next N results.
     */
    public readonly getNextN: (n: number) => Promise<Result[]>,
    /**
     * Freeing a query manually is required since its results live in the WebWorker.
     */
    public readonly free: () => void,
  ) {}
}

export default Query;
