import Result from './Result';

const Heap = require('heap');

class Results {
  resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

  add(results: Result[]): void {
    results.forEach((result) => this.resultHeap.push(result));
  }

  retrieve(n: number): Result[] {
    const minAmtResults = Math.min(n, this.resultHeap.size());
    const retrievedResults = [];
    for (let i = 0; i < minAmtResults; i += 1) {
      retrievedResults.push(this.resultHeap.pop());
    }
    return retrievedResults;
  }
}

export default Results;
