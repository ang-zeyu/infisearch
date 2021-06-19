// eslint-disable-next-line max-classes-per-file
import decodeVarInt from '../utils/varInt';
import TermInfo from '../results/TermInfo';

interface DocField {
  fieldId: number,
  fieldPositions: number[]
}

interface TermDoc {
  docId: number,
  fields: DocField[]
}

class PlIterator {
  td: TermDoc = { docId: 0, fields: [] };

  private idx = 0;

  constructor(private postingsList: PostingsList) {
    // eslint-disable-next-line prefer-destructuring
    this.td = postingsList.termDocs[0];
  }

  next(): void {
    // eslint-disable-next-line no-plusplus
    this.td = this.postingsList.termDocs[this.idx++];
  }
}

class PostingsList {
  termDocs: TermDoc[] = [];

  constructor(
    public readonly term: string,
    private readonly termInfo: TermInfo,
  ) {}

  async fetch(baseUrl: string): Promise<void> {
    const arrayBuffer = await (await fetch(`${baseUrl}/pl_${this.termInfo.postingsFileName}`)).arrayBuffer();
    const dataView = new DataView(arrayBuffer);

    let prevDocId = 0;
    let pos = this.termInfo.postingsFileOffset;
    for (let i = 0; i < this.termInfo.docFreq; i += 1) {
      const { value: docIdGap, newPos: posAfterDocId } = decodeVarInt(dataView, pos);
      pos = posAfterDocId;

      const termDoc: TermDoc = {
        docId: prevDocId + docIdGap,
        fields: [],
      };
      prevDocId = termDoc.docId;

      let isLast = 0;
      do {
        const nextInt = dataView.getUint8(pos);
        pos += 1;

        /* eslint-disable no-bitwise */
        const fieldId = nextInt & 0x7f;
        isLast = nextInt & 0x80;
        /* eslint-enable no-bitwise */

        const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(dataView, pos);
        pos = posAfterTermFreq;

        const fieldPositions = [];
        let prevPos = 0;
        for (let i = 0; i < fieldTermFreq; i += 1) {
          const { value: posGap, newPos: posAfterPosGap } = decodeVarInt(dataView, pos);
          pos = posAfterPosGap;

          const currPos = prevPos + posGap;
          fieldPositions.push(currPos);
          prevPos = currPos;
        }

        termDoc.fields[fieldId] = { fieldId, fieldPositions };
      } while (!isLast);

      this.termDocs.push(termDoc);
    }
  }

  getIt(): PlIterator {
    return new PlIterator(this);
  }
}

export default PostingsList;
