import Result from './Result';
import FieldInfo from './FieldInfo';
import PostingsList from '../PostingsList/PostingsList';
import Dictionary from '../Dictionary/Dictionary';
import QueryVector from './QueryVector';
import DocInfo from './DocInfo';

const Heap = require('heap');

class Query {
  private postingsLists: { [term: string]: PostingsList } = {};

  private resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

  private retrievePromise: Promise<void> = undefined;

  constructor(
    public readonly aggregatedTerms: string[],
    public readonly queryVectors: QueryVector[],
    private docInfo: DocInfo,
    private fieldInfo: FieldInfo,
    private dictionary: Dictionary,
    private baseUrl: string,
    postingsLists: PostingsList[],
  ) {
    postingsLists.forEach((postingsList) => {
      this.postingsLists[postingsList.term] = postingsList;
    });
  }

  private async populate(n: number): Promise<Result[]> {
    const minAmtResults = Math.min(n, this.resultHeap.size());
    const retrievedResults: Result[] = [];
    for (let i = 0; i < minAmtResults; i += 1) {
      retrievedResults.push(this.resultHeap.pop());
    }

    console.log(retrievedResults);
    await Promise.all(retrievedResults.map((result) => result.populate(this.baseUrl)));

    return retrievedResults;
  }

  async retrieve(n: number): Promise<Result[]> {
    if (this.retrievePromise) {
      await this.retrievePromise;
      return this.populate(n);
    }
    let resolve;
    this.retrievePromise = new Promise((res) => { resolve = res; });

    const N = this.docInfo.numDocs;

    const docScores: { [docId:number]: number } = {};
    const docPositions: { [docId: number]: number[][] } = {};

    const contenders: Map<number, { [fieldId: number]: number[] }>[][] = await Promise.all(
      this.queryVectors.map((queryVec) => Promise.all(
        queryVec.getAllTerms().map((term) => this.postingsLists[term].getDocs()),
      )),
    );

    // Tf-idf computation
    this.queryVectors.forEach((queryVec, queryVecIdx) => {
      Object.entries(queryVec.getAllTermsAndWeights()).forEach(([term, termWeight], queryVecTermIdx) => {
        const idf = Math.log10(N / this.dictionary.termInfo[term].docFreq);

        contenders[queryVecIdx][queryVecTermIdx].forEach((fields, docId) => {
          let wfTD = 0;

          docPositions[docId] = docPositions[docId] ?? [];
          for (let i = docPositions[docId].length; i <= queryVecIdx; i += 1) {
            docPositions[docId].push([]);
          }

          Object.entries(fields).forEach(([fieldId, positions]) => {
            const fieldIdInt = Number(fieldId);
            const fieldWeight = this.fieldInfo[fieldIdInt].weight;
            const fieldLen = this.docInfo.docLengths[docId][fieldIdInt];

            const wtd = 1 + Math.log10(positions.length);

            docPositions[docId][queryVecIdx].push(...positions);

            // with normalization and weighted zone scoring
            wfTD += ((wtd * idf) / fieldLen) * fieldWeight;
          });

          wfTD *= termWeight;

          docScores[docId] = (docScores[docId] ?? 0) + wfTD;
        });
      });
    });

    this.rankByTermProximity(docPositions, docScores);

    Object.entries(docScores).forEach(([docId, score]) => {
      const docIdInt = Number(docId);
      this.resultHeap.push(new Result(docIdInt, score, this.fieldInfo));
    });

    const populated = await this.populate(n);

    resolve();

    return populated;
  }

  private rankByTermProximity(
    docPositions: { [docId: number]: number[][] },
    docScores: { [docId: number]: number },
  ): void {
    if (this.queryVectors.length <= 1) {
      return;
    }

    const MIN_WINDOW_MAX_BOUND = 10000;
    const defaultScalingFactor = 1 + Math.log10(MIN_WINDOW_MAX_BOUND / this.queryVectors.length);
    console.log(`Default scaling factor ${defaultScalingFactor}`);
    Object.entries(docPositions).forEach(([docId, docQueryVecPositions]) => {
      const docIdInt = Number(docId);
      if (
        Object.values(docQueryVecPositions).filter((positions) => positions.length).length
          !== this.queryVectors.length
      ) {
        docScores[docIdInt] /= defaultScalingFactor;
        return;
      }

      const iteratorAndPos: { it: number, positions: number[] }[] = [];
      Object.values(docQueryVecPositions).forEach((positions) => {
        positions.sort();
        iteratorAndPos.push({ it: 0, positions });
      });

      const initialPositions = iteratorAndPos.map((itAndPos) => itAndPos.positions[itAndPos.it]);
      let minWindow = Math.max(...initialPositions) - Math.min(...initialPositions) + 1;
      while (iteratorAndPos.every((itAndPos) => itAndPos.it + 1 < itAndPos.positions.length)) {
        let minNextPos = Number.MAX_VALUE;
        let minNextPosIdx = Number.MAX_VALUE;
        iteratorAndPos.forEach((itAndPos, idx) => {
          if (itAndPos.positions[itAndPos.it + 1] < minNextPos) {
            minNextPos = itAndPos.positions[itAndPos.it + 1];
            minNextPosIdx = idx;
          }
        });

        iteratorAndPos[minNextPosIdx].it += 1;

        const currentPositions = iteratorAndPos.map((itAndPos) => itAndPos.positions[itAndPos.it]);
        const window = Math.max(...currentPositions) - Math.min(...currentPositions) + 1;
        minWindow = Math.min(minWindow, window);
      }

      // Scoring function for query term proximity
      minWindow = Math.min(MIN_WINDOW_MAX_BOUND, minWindow);
      const factor = 1 + Math.log10(minWindow / this.queryVectors.length);
      docScores[docIdInt] /= factor;

      console.log(`Scaling positions for ${docId} by factor ${factor}`);
      console.log(docQueryVecPositions);
    });
  }
}

export default Query;
