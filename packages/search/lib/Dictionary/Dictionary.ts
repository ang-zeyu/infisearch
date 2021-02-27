import * as levenshtein from 'fast-levenshtein';

import decodeVarInt from '../utils/varInt';
import TermInfo from '../results/TermInfo';
import QueryVector from '../results/QueryVector';

const PREFIX_FRONT_CODE = 123; // '{'
const SUBSEQUENT_FRONT_CODE = 125; // '}'

const BIGRAM_START_CHAR = '^';
const BIGRAM_END_CHAR = '$';

const CORRECTION_ALPHA = 0.85;
const SPELLING_CORRECTION_ALPHA = 0.7;

class Dictionary {
  setupPromise: Promise<void>;

  termInfo: {
    [term: string]: TermInfo
  } = Object.create(null);

  biGrams: {
    [biGram: string]: string[]
  } = Object.create(null);

  constructor(url) {
    this.setupPromise = this.setup(url);
  }

  async setup(url): Promise<void> {
    const dictionaryTablePromise = fetch(`${url}/dictionaryTable`, {
      method: 'GET',
    });

    const dictionaryStringBuffer = await (await fetch(`${url}/dictionaryString`, {
      method: 'GET',
    })).arrayBuffer();
    const dictionaryStringView = new DataView(dictionaryStringBuffer);

    const decoder = new TextDecoder();

    const dictionaryTableBuffer = await (await dictionaryTablePromise).arrayBuffer();
    const dictionaryTableView = new DataView(dictionaryTableBuffer);

    const tempTermInfos: {
      term: string
      docFreq: number
      postingsFileName: number
      postingsFileOffset: number
      postingsFileEndName: number
      postingsFileEndOffset: number
    }[] = [];

    let prevPostingsFileName = 0;
    let dictStringPos = 0;
    let frontCodingPrefix = '';
    for (let dictTablePos = 0; dictTablePos < dictionaryTableBuffer.byteLength;) {
      const postingsFileName = dictionaryTableView.getUint8(dictTablePos) + prevPostingsFileName;
      dictTablePos += 1;
      prevPostingsFileName = postingsFileName;

      const { value: docFreq, newPos: dictTablePos1 } = decodeVarInt(dictionaryTableView, dictTablePos);
      dictTablePos = dictTablePos1;

      const postingsFileOffset = dictionaryTableView.getUint16(dictTablePos, true);
      dictTablePos += 2;

      const termLen = dictionaryStringView.getUint8(dictStringPos);
      dictStringPos += 1;

      if (frontCodingPrefix) {
        if (dictionaryStringView.getUint8(dictStringPos) !== SUBSEQUENT_FRONT_CODE) {
          frontCodingPrefix = '';
        } else {
          dictStringPos += 1;
        }
      }

      let term = decoder.decode(dictionaryStringBuffer.slice(dictStringPos, dictStringPos + termLen));
      dictStringPos += termLen;

      if (frontCodingPrefix) {
        term = frontCodingPrefix + term;
      } else if (term.indexOf('{') !== -1) {
        [frontCodingPrefix] = term.split('{');

        // Redecode the full string, then remove the '{'
        term = decoder
          .decode(dictionaryStringBuffer.slice(dictStringPos - termLen, dictStringPos + 1))
          .replace('{', '');
        dictStringPos += 1;
      } else if (dictStringPos < dictionaryStringBuffer.byteLength
        && dictionaryStringView.getUint8(dictStringPos) === PREFIX_FRONT_CODE) {
        frontCodingPrefix = term;
        dictStringPos += 1;
      }

      // console.log(`${frontCodingPrefix} ${term}`);
      if (term.indexOf('{') !== -1 || term.indexOf('}') !== -1) {
        throw new Error(`Uh oh ${term}`);
      }

      tempTermInfos.push({
        term,
        docFreq,
        postingsFileName,
        postingsFileOffset,
        postingsFileEndName: postingsFileName,
        postingsFileEndOffset: Number.MAX_VALUE,
      });
    }

    const sentinelIdx = tempTermInfos.length - 1;
    for (let i = 0; i < sentinelIdx; i += 1) {
      this.termInfo[tempTermInfos[i].term] = tempTermInfos[i];
      this.termInfo[tempTermInfos[i].term].postingsFileEndName = tempTermInfos[i + 1].postingsFileName;
      this.termInfo[tempTermInfos[i].term].postingsFileEndOffset = tempTermInfos[i + 1].postingsFileOffset;
    }

    this.setupBigram();
  }

  private static getBiGrams(term: string): string[] {
    const biGrams = [];
    biGrams.push(BIGRAM_START_CHAR + term[0]);

    const end = term.length - 1;
    for (let i = 0; i < end; i += 1) {
      biGrams.push(term[i] + term[i + 1]);
    }

    biGrams.push(term[end] + BIGRAM_END_CHAR);

    return biGrams;
  }

  private setupBigram(): void {
    Object.keys(this.termInfo).forEach((term) => {
      Dictionary.getBiGrams(term).forEach((biGram) => {
        this.biGrams[biGram] = this.biGrams[biGram] ?? [];
        this.biGrams[biGram].push(term);
      });
    });
  }

  getTerms(queryTerm: string, doExpand: boolean): QueryVector {
    if (!this.termInfo[queryTerm]) {
      return this.getCorrectedTerms(queryTerm);
    }

    if (doExpand) {
      return this.getExpandedTerms(queryTerm);
    }

    const queryVec = new QueryVector();
    queryVec.addTerm(queryTerm, 1);
    return queryVec;
  }

  getCorrectedTerms(misSpelledTerm: string): QueryVector {
    const levenshteinCandidates = this.getTermCandidates(misSpelledTerm, true);

    const editDistances: { [term: string]: number } = Object.create(null);
    levenshteinCandidates.forEach((term) => {
      editDistances[term] = levenshtein.get(misSpelledTerm, term);
    });

    let minEditDistanceTerms = new QueryVector();
    let minEditDistance = 99999;
    Object.entries(editDistances).forEach(([term, editDistance]) => {
      if (editDistance >= 3) {
        return;
      }

      if (editDistance < minEditDistance) {
        minEditDistanceTerms = new QueryVector();
        minEditDistanceTerms.addTerm(term, 1);
        minEditDistance = editDistance;
      } else if (editDistance === minEditDistance) {
        minEditDistanceTerms.addTerm(term, 1);
      }
    });

    return minEditDistanceTerms;
  }

  getExpandedTerms(baseTerm: string): QueryVector {
    const queryVec = new QueryVector();
    queryVec.addTerm(baseTerm, 1);
    if (baseTerm.length < 4) {
      return queryVec;
    }

    const prefixCheckCandidates = this.getTermCandidates(baseTerm, false);

    const minBaseTermSubstring = baseTerm.substring(0, Math.floor(CORRECTION_ALPHA * baseTerm.length));
    prefixCheckCandidates.forEach((term) => {
      if (term.startsWith(minBaseTermSubstring) && term !== baseTerm) {
        queryVec.addTerm(term, 1 / (term.length - minBaseTermSubstring.length + 1));
      }
    });

    return queryVec;
  }

  private getTermCandidates(baseTerm: string, useJacard: boolean): string[] {
    const biGrams = Dictionary.getBiGrams(baseTerm);
    const minMatchingBiGrams = Math.floor(CORRECTION_ALPHA * biGrams.length);

    const candidates: { [term: string]: number } = Object.create(null);
    biGrams.forEach((biGram) => {
      if (!this.biGrams[biGram]) {
        return;
      }

      this.biGrams[biGram].forEach((term) => {
        candidates[term] = candidates[term] ? candidates[term] + 1 : 1;
      });
    });

    return Object.keys(candidates).filter((term) => (useJacard
      // (A intersect B) / (A union B)
      // For n-gram string, there are n + 1 bi-grams
      ? candidates[term] / (term.length + baseTerm.length + 2 - candidates[term]) >= SPELLING_CORRECTION_ALPHA
      : candidates[term] >= minMatchingBiGrams));
  }
}

export default Dictionary;
