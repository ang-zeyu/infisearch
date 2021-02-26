import Result from './Result';
import Storage from './Storage';
import FieldInfo from './FieldInfo';
import PostingsList from '../PostingsList/PostingsList';
import Dictionary from '../Dictionary/Dictionary';
import QueryVector from './QueryVector';

const Heap = require('heap');

class Query {
  private postingsLists: { [term: string]: PostingsList } = {};

  constructor(
    public readonly aggregatedTerms: string[],
    private readonly queryVectors: QueryVector[],
    private storages: {
      [baseName: string]: Storage
    },
    private docLengths: number[][],
    private fieldInfo: FieldInfo,
    private dictionary: Dictionary,
    postingsLists: PostingsList[],
  ) {
    postingsLists.forEach((postingsList) => {
      this.postingsLists[postingsList.term] = postingsList;
    });
  }

  async retrieve(n: number): Promise<Result[]> {
    const N = this.docLengths[0][0]; // first line is number of documents

    const docScores: { [docId:number]: number } = {};
    const docPositions: { [docId: number]: number[][] } = {};

    // Tf-idf computation
    this.queryVectors.forEach((queryVec, queryVecIdx) => {
      Object.entries(queryVec.termsAndWeights).forEach(([term, termWeight]) => {
        const postingsList = this.postingsLists[term];
        const idf = Math.log10(N / this.dictionary.termInfo[term].docFreq);

        const r = n * 3;
        // console.log(`${r} ${term}`);
        const nextRDocs = postingsList.getDocs(r);

        nextRDocs.forEach((fields, docId) => {
          let wfTD = 0;

          docPositions[docId] = docPositions[docId] ?? [];
          for (let i = docPositions[docId].length; i <= queryVecIdx; i += 1) {
            docPositions[docId].push([]);
          }

          Object.entries(fields).forEach(([fieldId, positions]) => {
            const fieldIdInt = Number(fieldId);
            const fieldWeight = this.fieldInfo[fieldIdInt].weight;
            const fieldLen = this.docLengths[docId][fieldIdInt - 1];

            const termFreq = positions.length;
            const wtd = 1 + Math.log10(termFreq);

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

    const resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

    Object.entries(docScores).forEach(([docId, score]) => {
      const docIdInt = Number(docId);
      resultHeap.push(new Result(docIdInt, score));
    });

    const minAmtResults = Math.min(n, resultHeap.size());
    const retrievedResults: Result[] = [];
    for (let i = 0; i < minAmtResults; i += 1) {
      retrievedResults.push(resultHeap.pop());
    }

    const docIds = retrievedResults.map((result) => result.docId);
    Object.values(this.postingsLists).forEach((postingsList) => postingsList.deleteDocs(docIds));

    await Promise.all(Object.values(this.storages).map((storage) => storage.populate(retrievedResults)));

    return retrievedResults;
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
