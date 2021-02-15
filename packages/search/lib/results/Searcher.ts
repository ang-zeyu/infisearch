import Result from './Result';
import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Results from './Results';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docInfo: Promise<string[]>;

  constructor(
    private url: string,
  ) {
    this.dictionary = new Dictionary(url);
    this.postingsListManager = new PostingsListManager(url, this.dictionary);
    this.docInfo = this.setupLinks();
  }

  async setupLinks(): Promise<string[]> {
    const text = await (await fetch(`${this.url}/docInfo.txt`, {
      method: 'GET',
      headers: {
        'Content-Type': 'text/plain',
      },
    })).text();

    return text.split('\n');
  }

  async getResults(query): Promise<Results> {
    const terms = query.split(/\s+/g);
    await this.dictionary.setupPromise;
    await this.postingsListManager.retrieve(terms);
    const docInfo = await this.docInfo;
    const N = parseInt(docInfo[0], 10);

    const docScores: { [docId:number]: number } = {};

    terms.forEach((term) => {
      if (!this.dictionary.termInfo[term]) {
        return;
      }

      const contenders = this.postingsListManager.getDocs(term);
      const idf = Math.log10(N / this.dictionary.termInfo[term].docFreq);
      // w_tq = 1;

      contenders.forEach((contender) => {
        const wtd = 1 + Math.log10(contender.termFreq);
        docScores[contender.docId] = (docScores[contender.docId] ? docScores[contender.docId] : 0)
          + wtd * idf;
      });
    });

    const results = new Results();
    results.add(Object.keys(docScores).map(Number).map((docId) => {
      docScores[docId] /= parseFloat(docInfo[docId]);
      return new Result(docId, docScores[docId], docInfo[docId * 2]);
    }));

    return results;
  }
}

export default Searcher;
