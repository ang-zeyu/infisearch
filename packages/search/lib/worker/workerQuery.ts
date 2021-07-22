import { Query } from '../../../librarian_common/pkg';
import { QueryPart } from '../parser/queryParser';

export default class WorkerQuery {
  constructor(
    public aggregatedTerms: string[],
    public queryParts: QueryPart[],
    private query: Query,
  ) {}

  getNextN(n: number): number[] {
    return this.query.get_next_n(n);
  }

  free() {
    this.query.free();
  }
}
