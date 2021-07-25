import Result from './Result';
import { FieldInfo } from './FieldInfo';
import { QueryPart } from '../parser/queryParser';
import { SearcherOptions } from './SearcherOptions';

const Heap = require('heap');

class Query {
  private resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

  private retrievePromise: Promise<void> = undefined;

  constructor(
    public readonly query: string,
    public readonly aggregatedTerms: string[],
    public readonly queryParts: QueryPart[],
    public readonly fieldInfos: FieldInfo[],
    public readonly options: SearcherOptions,
    public readonly fieldStoreBlockSize: number,
    public readonly retrieve: (n: number) => Promise<Result[]>,
    public readonly free: () => void,
  ) {}
}

export default Query;
