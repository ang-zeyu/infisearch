import * as levenshtein from 'fast-levenshtein';

import TermInfo from '../results/TermInfo';
import QueryVector from '../results/QueryVector';
import getTriGrams from './triGrams';

const CORRECTION_ALPHA = 0.85;
const SPELLING_CORRECTION_BASE_ALPHA = 0.625;

class Dictionary {
  termInfo: {
    [term: string]: TermInfo
  };

  triGrams: {
    [triGram: string]: string[]
  };

  setup(setupDictionaryUrl: string, url: string, numDocs: number): Promise<void> {
    return new Promise((resolve, reject) => {
      const w = new Worker(setupDictionaryUrl);
      w.onmessage = (ev) => {
        this.termInfo = ev.data.termInfo;
        this.triGrams = ev.data.triGrams;
        resolve();
      };

      w.onmessageerror = reject;

      w.postMessage({ url, numDocs });
    });
  }

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

  getCorrectedTerms(misSpelledTerm: string): string[] {
    const levenshteinCandidates = this.getTermCandidates(misSpelledTerm, true);

    const editDistances: { [term: string]: number } = Object.create(null);
    levenshteinCandidates.forEach((term) => {
      editDistances[term] = levenshtein.get(misSpelledTerm, term);
    });

    let minEditDistanceTerms = [];
    let minEditDistance = 99999;
    Object.entries(editDistances).forEach(([term, editDistance]) => {
      if (editDistance >= 3) {
        return;
      }

      if (editDistance < minEditDistance) {
        minEditDistanceTerms = [];
        minEditDistanceTerms.push(term);
        minEditDistance = editDistance;
      } else if (editDistance === minEditDistance) {
        minEditDistanceTerms.push(term);
      }
    });

    return minEditDistanceTerms;
  }

  getExpandedTerms(baseTerm: string): { [term: string]: number } {
    if (baseTerm.length < 4) {
      return Object.create(null);
    }

    const expandedTerms: { [term: string]: number } = Object.create(null);
    const prefixCheckCandidates = this.getTermCandidates(baseTerm, false);

    const minBaseTermSubstring = baseTerm.substring(0, Math.floor(CORRECTION_ALPHA * baseTerm.length));
    prefixCheckCandidates.forEach((term) => {
      if (term.startsWith(minBaseTermSubstring) && term !== baseTerm) {
        expandedTerms[term] = 1 / (term.length - minBaseTermSubstring.length + 1);
      }
    });

    return expandedTerms;
  }

  private getTermCandidates(baseTerm: string, useJacard: boolean): string[] {
    const triGrams = getTriGrams(baseTerm);
    const minMatchingTriGrams = Math.floor(CORRECTION_ALPHA * triGrams.length);

    const candidates: { [term: string]: number } = Object.create(null);
    triGrams.forEach((triGram) => {
      if (!this.triGrams[triGram]) {
        return;
      }

      this.triGrams[triGram].forEach((term) => {
        candidates[term] = candidates[term] ? candidates[term] + 1 : 1;
      });
    });

    return Object.keys(candidates).filter((term) => (useJacard
      // (A intersect B) / (A union B)
      // For n-gram string, there are n - 2 tri-grams
      ? (candidates[term] / (term.length + baseTerm.length - 4 - candidates[term]))
        >= SPELLING_CORRECTION_BASE_ALPHA
      : candidates[term] >= minMatchingTriGrams));
  }
}

export default Dictionary;
