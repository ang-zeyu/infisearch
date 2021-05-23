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

    let plIteratorsAndWeights = this.queryVectors
      .map((q) => Object.entries(q.getAllTermsAndWeights()).map(([term, weight]) => ({
        term, it: this.postingsLists[term].getIt(), weight,
      })))
      .reduce((acc, curr) => acc.concat(curr), [])
      .sort((a, b) => a.it.td.docId - b.it.td.docId);
    let queryVectorTermSets: Set<string>[];
    const doUseQueryTermProximityRanking = true;
    if (doUseQueryTermProximityRanking) {
      queryVectorTermSets = this.queryVectors.map((q) => new Set(q.getAllTerms()));
    }

    // DAAT traversal
    while (plIteratorsAndWeights.length > 0) {
      const { docId } = plIteratorsAndWeights[0].it.td;
      const result = new Result(Number(docId), 0, this.fieldInfo);

      let scalingFactor = 1;

      // Query term proximity ranking
      if (doUseQueryTermProximityRanking && queryVectorTermSets.length > 1) {
        const positionHeap: Heap<[number, number, number, number]> = new Heap((a, b) => a[0] - b[0]);
        for (let i = 0; i < plIteratorsAndWeights.length; i += 1) {
          const currFields = plIteratorsAndWeights[i].it.td.fields;
          for (let j = 0; j < currFields.length; j += 1) {
            if (!currFields[j] || !currFields[j].fieldPositions.length) {
              continue;
            }
            const { fieldPositions } = currFields[j];
            positionHeap.push([fieldPositions[0], i, j, 1]);
          }
        }

        const mergedPositions: [number, string][] = [];
        while (!positionHeap.empty()) {
          const top = positionHeap.pop();

          const docField = plIteratorsAndWeights[top[1]].it.td.fields[top[2]];
          if (top[3] < docField.fieldPositions.length) {
            positionHeap.push([docField.fieldPositions[top[3]], top[1], top[2], top[3] + 1]);
          }

          mergedPositions.push([top[0], plIteratorsAndWeights[top[1]].term]);
        }
        /* console.log(docId);
        console.log(mergedPositions); */

        let atQueryVec = 0;
        let minWindow = [];
        let minWindowLen = 2000000000;
        const currWindow = [];
        let currWindowLen = 2000000000;
        for (let i = 0; i < mergedPositions.length; i += 1) {
          if (queryVectorTermSets[atQueryVec].has(mergedPositions[i][1])) {
            // eslint-disable-next-line prefer-destructuring
            currWindow[atQueryVec] = mergedPositions[i][0];
            atQueryVec += 1;
          } else if (atQueryVec !== 0 && queryVectorTermSets[0].has(mergedPositions[i][1])) {
            // eslint-disable-next-line prefer-destructuring
            currWindow[0] = mergedPositions[i][0];
            atQueryVec = 1;
          } else {
            atQueryVec = 0;
          }

          if (atQueryVec >= queryVectorTermSets.length) {
            currWindowLen = Math.max(...currWindow) - Math.min(...currWindow);
            atQueryVec = 0;
            if (currWindowLen < minWindowLen) {
              minWindow = currWindow;
              minWindowLen = currWindowLen;
            }
          }
        }

        if (minWindowLen < 10000) {
          scalingFactor = 1 + 7 / (10 + minWindowLen);
          console.log(`Scaling ${docId} by ${scalingFactor}, minWindowLen ${minWindowLen}`);
        }
      }

      for (let i = 0; i < plIteratorsAndWeights.length; i += 1) {
        const curr = plIteratorsAndWeights[i];
        if (curr.it.td.docId !== docId) {
          break;
        }

        const currDocFields = curr.it.td.fields;
        let score = 0;
        for (let j = 0; j < currDocFields.length; j += 1) {
          if (!curr.it.td.fields[j]) {
            // eslint-disable-next-line no-continue
            continue;
          }

          const currDocField = currDocFields[j];
          const fieldInfo = this.fieldInfo[currDocField.fieldId];
          const fieldLenFactor = this.docInfo.docLengthFactors[docId][currDocField.fieldId];
          const fieldTermFreq = currDocField.fieldPositions.length;

          score += ((fieldTermFreq * (fieldInfo.k + 1))
            / (fieldTermFreq + fieldInfo.k * (1 - fieldInfo.b + fieldInfo.b * (fieldLenFactor))))
            * fieldInfo.weight;
        }

        score *= this.dictionary.termInfo[curr.term].idf * curr.weight;
        result.score += score;

        curr.it.next();
      }

      result.score *= scalingFactor;
      this.resultHeap.push(result);

      plIteratorsAndWeights = plIteratorsAndWeights
        .filter((x) => x.it.td)
        .sort((a, b) => a.it.td.docId - b.it.td.docId);
    }

    const populated = await this.populate(n);

    resolve();

    return populated;
  }
}

export default Query;
