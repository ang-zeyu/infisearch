import Result from './Result';
import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Results from './Results';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docLengths: Promise<number[][]>;

  private fieldInfo: Promise<{
    [id: number]: {
      name: string,
      storage: string,
      storageParams: { [param: string]: any },
      weight: number
    }
  }>;

  constructor(
    private url: string,
  ) {
    this.dictionary = new Dictionary(url);
    this.postingsListManager = new PostingsListManager(url, this.dictionary);
    this.docLengths = this.setupDocLengths();
    this.fieldInfo = this.setupFieldInfo();
  }

  async setupDocLengths(): Promise<number[][]> {
    const text = await (await fetch(`${this.url}/docInfo.txt`, {
      method: 'GET',
      headers: {
        'Content-Type': 'text/plain',
      },
    })).text();

    const docLengths = [];
    text.split('\n').forEach((line) => {
      docLengths.push(line.split(',').map(parseFloat));
    });

    return docLengths;
  }

  async setupFieldInfo() {
    const json = await (await fetch(`${this.url}/fieldInfo.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    Object.keys(json).forEach((fieldName) => {
      json[json[fieldName].id] = json[fieldName];
      json[json[fieldName].id].name = fieldName;
      delete json[json[fieldName].id].id;
      delete json[fieldName];
    });
    console.log(json);

    return json;
  }

  async getResults(query): Promise<Results> {
    const terms = query.split(/\s+/g);
    await this.dictionary.setupPromise;
    await this.postingsListManager.retrieve(terms);
    const docLengths = await this.docLengths;
    const fieldInfo = await this.fieldInfo;
    const N = docLengths[0][0]; // first line is number of documents

    const docScores: { [docId:number]: { [fieldId: number]: number } } = {};

    terms.forEach((term) => {
      if (!this.dictionary.termInfo[term]) {
        return;
      }

      const postingsList = this.postingsListManager.getDocs(term);
      const idf = Math.log10(N / this.dictionary.termInfo[term].docFreq);

      Object.entries(postingsList.termFreqs).forEach(([docId, fields]) => {
        const docIdInt = Number(docId);
        docScores[docIdInt] = docScores[docIdInt] ?? {};

        Object.entries(fields).forEach(([fieldId, termFreq]) => {
          const wtd = 1 + Math.log10(termFreq);
          docScores[docIdInt][fieldId] = (docScores[docIdInt][fieldId] ?? 0) + wtd * idf;
        });
      });
    });

    const results = new Results(fieldInfo, this.url);
    results.add(Object.entries(docScores).map(([docId, fieldScores]) => {
      const docIdInt = Number(docId);
      let docScore = 0;

      Object.entries(fieldScores).forEach(([fieldId, fieldScore]) => {
        const fieldIdInt = Number(fieldId);
        const fieldWeight = fieldInfo[fieldIdInt].weight;
        const fieldLen = docLengths[docIdInt][fieldIdInt - 1];
        docScore += ((fieldScore / fieldLen) * fieldWeight);
      });

      return new Result(docIdInt, docScore);
    }));

    return results;
  }
}

export default Searcher;
