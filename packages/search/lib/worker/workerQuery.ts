import { QueryPart } from '../parser/queryParser';

export default class WorkerQuery {
  constructor(
    public _mrlQueryParts: QueryPart[],
    public _mrlResultsTotal: number,
    private _mrlQuery: any,
  ) {}

  _mrlGetNextN(n: number): number[] {
    return Array.from(this._mrlQuery.get_next_n(n));
  }

  _mrlFree() {
    this._mrlQuery.free();
  }
}
