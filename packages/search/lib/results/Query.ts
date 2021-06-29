import Result from './Result';
import FieldInfo from './FieldInfo';
import { PostingsList, TermPostingsList } from '../PostingsList/PostingsList';
import Dictionary from '../Dictionary/Dictionary';
import DocInfo from './DocInfo';
import { QueryPart } from '../parser/queryParser';

const Heap = require('heap');

class Query {
  private resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

  private retrievePromise: Promise<void> = undefined;

  private isFreeTextQuery: boolean = true;

  constructor(
    public readonly aggregatedTerms: string[],
    public readonly queryParts: QueryPart[],
    private postingsLists: PostingsList[],
    private docInfo: DocInfo,
    private fieldInfos: FieldInfo[],
    private dictionary: Dictionary,
    private baseUrl: string,
  ) {
    postingsLists.forEach((postingsList) => {
      this.isFreeTextQuery = this.isFreeTextQuery && (postingsList instanceof TermPostingsList);
    });
  }

  private async populate(n: number): Promise<Result[]> {
    const minAmtResults = Math.min(n, this.resultHeap.size());
    const retrievedResults: Result[] = [];
    for (let i = 0; i < minAmtResults; i += 1) {
      retrievedResults.push(this.resultHeap.pop());
    }

    // console.log(retrievedResults);
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

    let plIterators = this.postingsLists
      .map((postingsList) => postingsList.getIt())
      .filter((x) => x.td)
      .sort((a, b) => a.td.docId - b.td.docId);

    const doUseQueryTermProximityRanking = true;

    // Document-at-a-time (DAAT) traversal

    const topNMinHeap: Heap<{ docId: number; score: number }> = new Heap(
      (r1: Result, r2: Result) => r1.score - r2.score,
    );

    while (plIterators.length > 0) {
      let { docId: pivotDocId } = plIterators[0].td;

      // WAND algorithm
      if (this.isFreeTextQuery && topNMinHeap.size() >= n) {
        const nthHighestScore = topNMinHeap.peek();
        let wandAcc = 0;
        let pivotListIdx = 0;

        for (; pivotListIdx < plIterators.length; pivotListIdx += 1) {
          wandAcc += plIterators[pivotListIdx].pl.termInfo.maxTermScore;
          if (wandAcc > nthHighestScore.score) {
            pivotDocId = plIterators[pivotListIdx].td.docId;
            break;
          }
        }

        if (wandAcc < nthHighestScore.score) {
          break;
        }

        for (let i = 0; i < pivotListIdx; i += 1) {
          const curr = plIterators[i];
          while (curr.td && curr.td.docId < pivotDocId) {
            curr.next();
          }
        }

        plIterators = plIterators.filter((pl) => pl.td);
      }

      const result = new Result(Number(pivotDocId), 0, this.fieldInfos);

      let scalingFactor = 1;

      // Query term proximity ranking
      if (doUseQueryTermProximityRanking) {
        const plIteratorsForProximityRanking = plIterators
          .filter((plIt) => plIt.pl.includeInProximityRanking && plIt.td.docId === pivotDocId);

        if (plIteratorsForProximityRanking.length > 1) {
          const positionHeap: Heap<[
            number, // pos
            number, // plIteratorsIdx
            number, // plIterators field idx
            number, // plIterators field fieldPositions **next** idx
          ]> = new Heap((a, b) => a[0] - b[0]);
          for (let i = 0; i < plIteratorsForProximityRanking.length; i += 1) {
            const currFields = plIteratorsForProximityRanking[i].td.fields;
            for (let j = 0; j < currFields.length; j += 1) {
              if (!currFields[j] || !currFields[j].fieldPositions.length) {
                continue;
              }
              positionHeap.push([currFields[j].fieldPositions[0], i, j, 1]);
            }
          }

          // Merge the disjoint fields positions into one
          // [pos, plIteratorsIdx][]
          const mergedPositions: [number, number][] = [];
          while (!positionHeap.empty()) {
            const top = positionHeap.pop();

            const docField = plIteratorsForProximityRanking[top[1]].td.fields[top[2]];
            if (top[3] < docField.fieldPositions.length) {
              positionHeap.push([docField.fieldPositions[top[3]], top[1], top[2], top[3] + 1]);
            }

            mergedPositions.push([
              top[0],
              top[1],
            ]);
          }

          let nextExpected = 0;
          let minWindowLen = 2000000000;
          const currWindow = [];
          let currWindowLen = 2000000000;
          for (let i = 0; i < mergedPositions.length; i += 1) {
            if (nextExpected === mergedPositions[i][1]) {
              // Continue the match
              [currWindow[nextExpected]] = mergedPositions[i];
              nextExpected += 1;
            } else if (nextExpected !== 0 && mergedPositions[i][1] === 0) {
              // Restart the match from 1
              [currWindow[0]] = mergedPositions[i];
              nextExpected = 1;
            } else {
              // Restart the match from 0
              nextExpected = 0;
            }

            if (nextExpected >= plIteratorsForProximityRanking.length) {
              currWindowLen = Math.max(...currWindow) - Math.min(...currWindow);
              nextExpected = 0;
              minWindowLen = Math.min(currWindowLen, minWindowLen);
            }
          }

          if (minWindowLen < 10000) {
            scalingFactor = 1 + 7 / (10 + minWindowLen);
            // console.log(`Scaling ${pivotDocId} by ${scalingFactor}, minWindowLen ${minWindowLen}`);
          }
        }
      }

      for (let i = 0; i < plIterators.length; i += 1) {
        const curr = plIterators[i];
        if (curr.td.docId === pivotDocId) {
          const currDocFields = curr.td.fields;
          let score = 0;

          for (let j = 0; j < currDocFields.length; j += 1) {
            if (curr.td.fields[j]) {
              const currDocField = currDocFields[j];
              const fieldInfo = this.fieldInfos[currDocField.fieldId];
              const fieldLenFactor = this.docInfo.docLengthFactors[pivotDocId][currDocField.fieldId];
              const fieldTermFreq = currDocField.fieldPositions.length;

              score += ((fieldTermFreq * (fieldInfo.k + 1))
                / (fieldTermFreq + fieldInfo.k * (1 - fieldInfo.b + fieldInfo.b * (fieldLenFactor))))
                * fieldInfo.weight;
            }
          }

          score *= curr.pl.termInfo.idf * curr.pl.weight;
          result.score += score;

          curr.next();
        }
      }

      if (topNMinHeap.size() < n) {
        topNMinHeap.push({ docId: result.docId, score: result.score });
      } else if (result.score > topNMinHeap.peek().score) {
        topNMinHeap.replace({ docId: result.docId, score: result.score });
      }

      result.score *= scalingFactor;
      this.resultHeap.push(result);

      plIterators = plIterators
        .filter((x) => x.td)
        .sort((a, b) => a.td.docId - b.td.docId);
    }

    console.log('Total num results');
    console.log(this.resultHeap.size());

    const populated = await this.populate(n);

    resolve();

    return populated;
  }
}

export default Query;
