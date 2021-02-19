import Result from './Result';
import Storage from './Storage';

const Heap = require('heap');

class Query {
  resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

  constructor(
    public readonly queriedTerms: string[],
    private storages: {
      [baseName: string]: Storage
    },
  ) {}

  add(results: Result[]): void {
    results.forEach((result) => this.resultHeap.push(result));
  }

  async retrieve(n: number): Promise<Result[]> {
    const minAmtResults = Math.min(n, this.resultHeap.size());
    const retrievedResults: Result[] = [];
    for (let i = 0; i < minAmtResults; i += 1) {
      retrievedResults.push(this.resultHeap.pop());
    }

    await Promise.all(Object.values(this.storages).map((storage) => storage.populate(retrievedResults)));

    return retrievedResults;
  }
}

export default Query;
