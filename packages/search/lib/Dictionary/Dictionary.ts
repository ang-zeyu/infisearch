import * as levenshtein from 'fast-levenshtein';

import decodeVarInt from '../utils/varInt';

const PREFIX_FRONT_CODE = 42; // '*'
const SUBSEQUENT_FRONT_CODE = 38; // '&'

const BIGRAM_START_CHAR = '^';
const BIGRAM_END_CHAR = '$';

const SPELLING_CORRECTION_ALPHA = 0.8;

class Dictionary {
  setupPromise: Promise<void>;

  termInfo: {
    [term: string]: {
      postingsFileName: number
      docFreq: number
      postingsFileLength: number
      postingsFileOffset: number
    }
  } = Object.create(null);

  biGrams: {
    [biGram: string]: string[]
  } = Object.create(null);

  constructor(url) {
    this.setupPromise = this.setup(url);
  }

  async setup(url): Promise<void> {
    const dictionaryTablePromise = fetch(`${url}/dictionaryTable.txt`, {
      method: 'GET',
    });

    const dictionaryStringBuffer = await (await fetch(`${url}/dictionaryString.txt`, {
      method: 'GET',
    })).arrayBuffer();
    const dictionaryStringView = new DataView(dictionaryStringBuffer);

    const decoder = new TextDecoder();

    const dictionaryTableBuffer = await (await dictionaryTablePromise).arrayBuffer();
    const dictionaryTableView = new DataView(dictionaryTableBuffer);

    let prevPostingsFileName = 0;
    let dictStringPos = 0;
    let frontCodingPrefix = '';
    for (let dictTablePos = 0; dictTablePos < dictionaryTableBuffer.byteLength;) {
      const postingsFileName = dictionaryTableView.getUint8(dictTablePos) + prevPostingsFileName;
      dictTablePos += 1;
      prevPostingsFileName = postingsFileName;

      const { value: docFreq, newPos: dictTablePos1 } = decodeVarInt(dictionaryTableView, dictTablePos);
      dictTablePos = dictTablePos1;

      const {
        value: postingsFileLength, newPos: dictTablePos2,
      } = decodeVarInt(dictionaryTableView, dictTablePos);
      dictTablePos = dictTablePos2;

      const {
        value: postingsFileOffset, newPos: dictTablePos3,
      } = decodeVarInt(dictionaryTableView, dictTablePos);
      dictTablePos = dictTablePos3;

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
      } else if (term.indexOf('*') !== -1) {
        [frontCodingPrefix] = term.split('*');

        const suffixStartPos = dictStringPos - termLen + frontCodingPrefix.length + 1;
        const suffixEndPos = dictStringPos + 1;
        term = frontCodingPrefix
          + decoder.decode(dictionaryStringBuffer.slice(suffixStartPos, suffixEndPos));
        dictStringPos += 1;
      } else if (dictStringPos < dictionaryStringBuffer.byteLength
        && dictionaryStringView.getUint8(dictStringPos) === PREFIX_FRONT_CODE) {
        frontCodingPrefix = term;
        dictStringPos += 1;
      }

      this.termInfo[term] = {
        postingsFileName,
        docFreq,
        postingsFileLength,
        postingsFileOffset,
      };
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

  getTerm(queryTerm: string): string {
    if (this.termInfo[queryTerm]) {
      return queryTerm;
    }

    return this.getCorrectedTerm(queryTerm);
  }

  getCorrectedTerm(misSpelledTerm: string): string {
    const biGrams = Dictionary.getBiGrams(misSpelledTerm);
    const levenshteinCandidates: { [term: string]: number } = Object.create(null);
    biGrams.forEach((biGram) => {
      this.biGrams[biGram].forEach((term) => {
        levenshteinCandidates[term] = (levenshteinCandidates[term] ?? 0) + 1;
      });
    });

    const minMatchingBiGrams = Math.floor(SPELLING_CORRECTION_ALPHA * biGrams.length);
    const editDistances: { [term: string]: number } = Object.create(null);
    Object.entries(levenshteinCandidates).forEach(([term, numMatching]) => {
      if (numMatching < minMatchingBiGrams) {
        return;
      }

      editDistances[term] = levenshtein.get(misSpelledTerm, term);
    });

    let minEditDistanceTerm = '';
    let minEditDistance = 99999;
    Object.entries(editDistances).forEach(([term, editDistance]) => {
      if (editDistance < minEditDistance) {
        minEditDistanceTerm = term;
        minEditDistance = editDistance;
      }
    });
    console.log(minEditDistanceTerm);

    return minEditDistanceTerm;
  }
}

export default Dictionary;
