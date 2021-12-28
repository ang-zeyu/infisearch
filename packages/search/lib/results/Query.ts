import Result from './Result';
import { QueryPart } from '../parser/queryParser';

class Query {
  constructor(
    public readonly query: string,
    public readonly searchedTerms: string[],
    public readonly queryParts: QueryPart[],
    public readonly getNextN: (n: number) => Promise<Result[]>,
    public readonly free: () => void,
  ) {}
}

export default Query;
