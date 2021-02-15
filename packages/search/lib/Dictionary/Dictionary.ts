const PREFIX_FRONT_CODE = 42; // '*'
const SUBSEQUENT_FRONT_CODE = 38; // '&'

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

      const docFreq = dictionaryTableView.getUint32(dictTablePos, true);
      dictTablePos += 4;

      const postingsFileLength = dictionaryTableView.getUint32(dictTablePos, true);
      dictTablePos += 4;

      const postingsFileOffset = dictionaryTableView.getUint32(dictTablePos, true);
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
      } else if (term.indexOf('*') !== -1) {
        [frontCodingPrefix] = term.split('*');

        const suffixStartPos = dictStringPos - termLen + frontCodingPrefix.length;
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
  }
}

export default Dictionary;
