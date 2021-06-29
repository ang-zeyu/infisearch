import TermInfo from '../results/TermInfo';

interface PromiseResolvePair<T> {
  resolve: Function;
  promise: Promise<T>;
}

export default class Dictionary {
  w: Worker;

  private termInfo: {
    [term: string]: PromiseResolvePair<TermInfo>
  } = Object.create(null);

  private correctedTerm: {
    [term: string]: PromiseResolvePair<string>
  } = Object.create(null);

  private termsExpanded: {
    [term: string]: PromiseResolvePair<{ [term: string]: number }>
  } = Object.create(null);

  constructor(setupDictionaryUrl: string) {
    this.w = new Worker(setupDictionaryUrl);
    this.w.onmessage = (ev) => {
      if (ev.data.term) {
        const { term, termInfo } = ev.data;
        this.termInfo[term].resolve(termInfo);
      } else if (ev.data.termToCorrect) {
        const { termToCorrect, correctedTerm } = ev.data;
        this.correctedTerm[termToCorrect].resolve(correctedTerm);
      } else if (ev.data.termToExpand) {
        const { termToExpand, termsExpanded } = ev.data;
        this.termsExpanded[termToExpand].resolve(termsExpanded);
      }
    };
    this.w.onmessageerror = (ev) => { console.log(ev); };
  }

  refreshPromisePair(base: string, key: string, message: any): Promise<void> {
    return new Promise((resolveOuter) => {
      if (this[base][key]) {
        resolveOuter();
        return;
      }
      this[base][key] = {
        resolve: undefined,
        promise: undefined,
      };
      this[base][key].promise = new Promise((resolve) => {
        this[base][key].resolve = resolve;
        // post message only when resolve is set
        // otherwise it the worker may resolve earlier without a resolve() function to call
        this.w.postMessage(message);
        resolveOuter();
      });
    });
  }

  setup(url: string, numDocs: number) {
    this.w.postMessage({ url, numDocs });
  }

  async getTermInfo(term: string): Promise<TermInfo> {
    if (!this.termInfo[term]) {
      await this.refreshPromisePair('termInfo', term, { term });
    }
    return this.termInfo[term].promise;
  }

  async getBestCorrectedTerm(termToCorrect: string): Promise<string> {
    if (!this.correctedTerm[termToCorrect]) {
      await this.refreshPromisePair('correctedTerm', termToCorrect, { termToCorrect });
    }
    return this.correctedTerm[termToCorrect].promise;
  }

  async getExpandedTerms(termToExpand: string): Promise<{ [term: string]: number }> {
    if (!this.termsExpanded[termToExpand]) {
      await this.refreshPromisePair('termsExpanded', termToExpand, { termToExpand });
    }

    return this.termsExpanded[termToExpand].promise;
  }

  /*
  getTerms(queryTerm: string, doExpand: boolean): QueryVector {
    const queryVec = new QueryVector();

    if (!this.termInfo[queryTerm]) {
      this.getCorrectedTerms(queryTerm).forEach((term) => {
        queryVec.addCorrectedTerm(term, 1);
      });
      queryVec.mainTerm = queryTerm;

      return queryVec;
    }

    queryVec.setTerm(queryTerm, 1);

    if (doExpand) {
      Object.entries(this.getExpandedTerms(queryTerm)).forEach(([term, weight]) => {
        queryVec.addExpandedTerm(term, weight);
      });
    }

    return queryVec;
  }
  */
}
