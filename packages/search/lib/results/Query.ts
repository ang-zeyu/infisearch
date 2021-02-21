import Result from './Result';
import Storage from './Storage';
import FieldInfo from './FieldInfo';
import PostingsList from '../PostingsList/PostingsList';
import Dictionary from '../Dictionary/Dictionary';

const Heap = require('heap');

class Query {
  private postingsLists: { [term: string]: PostingsList } = {};

  constructor(
    public readonly aggregatedTerms: string[],
    private readonly queriedTerms: string[][],
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

    // Tf-idf computation
    this.queriedTerms.forEach((terms) => {
      terms.forEach((term, idx) => {
        const postingsList = this.postingsLists[term];
        const idf = Math.log10(N / this.dictionary.termInfo[term].docFreq);

        const r = n * 2;
        // console.log(`${r} ${term}`);
        const nextRDocs = postingsList.getDocs(r);

        nextRDocs.forEach((fields, docId) => {
          let wfTD = 0;

          Object.entries(fields).forEach(([fieldId, positions]) => {
            const fieldIdInt = Number(fieldId);
            const fieldWeight = this.fieldInfo[fieldIdInt].weight;
            const fieldLen = this.docLengths[docId][fieldIdInt - 1];

            const termFreq = positions.length;
            const wtd = 1 + Math.log10(termFreq);

            // with normalization and weighted zone scoring
            wfTD += ((wtd * idf) / fieldLen) * fieldWeight;
          });

          if (idx !== 0) {
            wfTD *= 0.5;
          }

          docScores[docId] = (docScores[docId] ?? 0) + wfTD;
        });
      });
    });

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
}

export default Query;
