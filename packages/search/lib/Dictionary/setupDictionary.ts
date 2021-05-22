import decodeVarInt from '../utils/varInt';
import getTriGrams from './triGrams';
import TermInfo from '../results/TermInfo';

const PREFIX_FRONT_CODE = 123; // '{'
const SUBSEQUENT_FRONT_CODE = 125; // '}'

async function getTermInfos(url: string, numDocs: number): Promise<{ [term: string]: TermInfo }> {
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

  const termInfo: { [term: string]: TermInfo } = Object.create(null);

  let prevPostingsFileName = -1;
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

    const maxTermScore = dictionaryTableView.getFloat32(dictTablePos, true);
    dictTablePos += 4;

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

    termInfo[term] = {
      docFreq,
      idf: Math.log(1 + (numDocs - docFreq + 0.5) / (docFreq + 0.5)),
      maxTermScore,
      postingsFileName,
      postingsFileOffset,
    };
  }

  return termInfo;
}

function setupTrigrams(termInfo: { [term: string]: TermInfo }): { [triGram: string]: string[] } {
  const triGrams: { [triGram: string]: string[] } = Object.create(null);
  Object.keys(termInfo).forEach((term) => {
    getTriGrams(term).forEach((triGram) => {
      triGrams[triGram] = triGrams[triGram] ?? [];
      triGrams[triGram].push(term);
    });
  });

  return triGrams;
}

onmessage = async function setupDictionary(ev) {
  const { url, numDocs } = ev.data;

  const termInfo = await getTermInfos(url, numDocs);
  const triGrams = setupTrigrams(termInfo);

  postMessage({
    termInfo,
    triGrams,
  });
};
