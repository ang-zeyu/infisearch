import Result from './Result';
import storageMap from './Storage';

const Heap = require('heap');

class Results {
  resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

  constructor(
    private fieldInfo: {
      [id: number]: {
        name: string,
        storage: string,
        storageParams: { [param: string]: any },
        weight: number
      }
    },
    private baseUrl: string,
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

    await Promise.all(Object.values(this.fieldInfo).map((info) => {
      const retrieve = storageMap[info.storage];
      return retrieve(retrievedResults, this.baseUrl, info.name, info.storageParams);
    }));

    return retrievedResults;
  }
}

export default Results;
