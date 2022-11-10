import { QueryPart } from '../parser/queryParser';

export default class WorkerQuery {
  constructor(
    public _mrlQueryParts: QueryPart[],
    public _mrlResultsTotal: number,
    private _mrlQuery: any,
  ) {}

  _mrlGetNextN(n: number): ArrayBuffer {
    return this._mrlQuery.get_next_n(n).buffer;
  }

  _mrlFree() {
    this._mrlQuery.free();
  }
}
