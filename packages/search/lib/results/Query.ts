import Result from './Result';
import { QueryPart } from '../parser/queryParser';

class Query {
  constructor(
    public readonly query: string,
    public readonly aggregatedTerms: string[],
    public readonly queryParts: QueryPart[],
    public readonly retrieve: (n: number) => Promise<Result[]>,
    public readonly free: () => void,
  ) {}
}

export default Query;
