import Result from './Result';
import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Results from './Results';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docLengths: Promise<number[]>;

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

  async setupDocLengths(): Promise<number[]> {
    const text = await (await fetch(`${this.url}/docInfo.txt`, {
      method: 'GET',
      headers: {
        'Content-Type': 'text/plain',
      },
    })).text();

    return text.split('\n').map((x) => parseFloat(x));
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

    return json;
  }

  async getResults(query): Promise<Results> {
    const terms = query.split(/\s+/g);
    await this.dictionary.setupPromise;
    await this.postingsListManager.retrieve(terms);
    const docLengths = await this.docLengths;
    const fieldInfo = await this.fieldInfo;
    const N = docLengths[0];

    const docScores: { [docId:number]: number } = {};

    terms.forEach((term) => {
      if (!this.dictionary.termInfo[term]) {
        return;
      }

      const postingsList = this.postingsListManager.getDocs(term);
      const idf = Math.log10(N / this.dictionary.termInfo[term].docFreq);

      Object.entries(postingsList.termFreqs).forEach(([docId, fields]) => {
        let totalTermFreq = 0;
        const docIdInt = Number(docId);

        Object.entries(fields).forEach(([fieldId, termFreq]) => {
          totalTermFreq += termFreq * fieldInfo[Number(fieldId)].weight;
        });

        const wtd = 1 + Math.log10(totalTermFreq);
        const tfidf = wtd * idf;
        docScores[docIdInt] = (docScores[docIdInt] ?? 0) + tfidf;
      });
    });

    const results = new Results(fieldInfo, this.url);
    results.add(Object.keys(docScores).map(Number).map((docId) => {
      docScores[docId] /= docLengths[docId];
      return new Result(docId, docScores[docId]);
    }));

    return results;
  }
}

export default Searcher;
