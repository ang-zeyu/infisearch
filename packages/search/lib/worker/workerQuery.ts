import { QueryPart } from '../parser/queryParser';

export default class WorkerQuery {
  constructor(
    public _mrlSearchedTerms: string[],
    public _mrlQueryParts: QueryPart[],
    private _mrlQuery: any,
  ) {}

  _mrlGetNextN(n: number): number[] {
    return Array.from(this._mrlQuery.get_next_n(n));
  }

  _mrlFree() {
    this._mrlQuery.free();
  }
}
