import { QueryPart } from '../parser/queryParser';

export default class WorkerQuery {
  constructor(
    public searchedTerms: string[],
    public queryParts: QueryPart[],
    private query: any,
  ) {}

  getNextN(n: number): number[] {
    return this.query.get_next_n(n);
  }

  free() {
    this.query.free();
  }
}
