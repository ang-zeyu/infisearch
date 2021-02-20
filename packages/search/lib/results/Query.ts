import Result from './Result';
import Storage from './Storage';
import PostingsListManager from '../PostingsList/PostingsListManager';
import FieldInfo from './FieldInfo';
import PostingsList from '../PostingsList/PostingsList';
import Dictionary from '../Dictionary/Dictionary';

const Heap = require('heap');

class Query {
  private postingsLists: { [term: string]: PostingsList } = {};

  constructor(
    public readonly queriedTerms: string[],
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

    const docScores: { [docId:number]: { [fieldId: number]: number } } = {};

    const r = n;

    // Tf-idf computation
    this.queriedTerms.forEach((term) => {
      const postingsList = this.postingsLists[term];
      const nextRDocs = postingsList.getDocs(r);
      const idf = Math.log10(N / this.dictionary.termInfo[term].docFreq);

      nextRDocs.forEach((fields, docId) => {
        docScores[docId] = docScores[docId] ?? {};

        Object.entries(fields).forEach(([fieldId, positions]) => {
          const termFreq = positions.length;
          const wtd = 1 + Math.log10(termFreq);
          docScores[docId][fieldId] = (docScores[docId][fieldId] ?? 0) + wtd * idf;
        });
      });
    });

    const resultHeap: Heap<Result> = new Heap((r1: Result, r2: Result) => r2.score - r1.score);

    // Normalization, weighted zone scoring
    Object.entries(docScores).forEach(([docId, fieldScores]) => {
      const docIdInt = Number(docId);
      let finalDocScore = 0;

      Object.entries(fieldScores).forEach(([fieldId, fieldScore]) => {
        const fieldIdInt = Number(fieldId);
        const fieldWeight = this.fieldInfo[fieldIdInt].weight;
        const fieldLen = this.docLengths[docIdInt][fieldIdInt - 1];
        finalDocScore += ((fieldScore / fieldLen) * fieldWeight);
      });

      resultHeap.push(new Result(docIdInt, finalDocScore));
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
