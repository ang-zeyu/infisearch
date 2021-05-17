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

  private docCount: number = 0;

  constructor(private view: DataView, private bufferPos: number, private docFreq: number) {
    this.next();
  }

  next(): TermDoc {
    this.docCount += 1;
    if (this.docCount > this.docFreq) {
      this.td = undefined;
      return;
    }

    const { value: docIdGap, newPos: posAfterDocId } = decodeVarInt(this.view, this.bufferPos);
    this.bufferPos = posAfterDocId;

    const termDoc: TermDoc = {
      docId: this.td.docId + docIdGap,
      fields: [],
    };

    let isLast = 0;
    do {
      const nextInt = this.view.getUint8(this.bufferPos);
      this.bufferPos += 1;

      /* eslint-disable no-bitwise */
      const fieldId = nextInt & 0x7f;
      isLast = nextInt & 0x80;
      /* eslint-enable no-bitwise */

      const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(this.view, this.bufferPos);
      this.bufferPos = posAfterTermFreq;

      const fieldPositions = [];
      let prevPos = 0;
      for (let i = 0; i < fieldTermFreq; i += 1) {
        const { value: posGap, newPos: posAfterPosGap } = decodeVarInt(this.view, this.bufferPos);
        this.bufferPos = posAfterPosGap;

        const currPos = prevPos + posGap;
        fieldPositions.push(currPos);
        prevPos = currPos;
      }

      termDoc.fields[fieldId] = { fieldId, fieldPositions };
    } while (!isLast);

    this.td = termDoc;

    return this.td;
  }
}

class PostingsList {
  private arrayBuffer: ArrayBuffer;

  constructor(
    public readonly term: string,
    private readonly url: string,
    private readonly termInfo: TermInfo,
  ) {}

  async fetch(): Promise<void> {
    this.arrayBuffer = await (await fetch(`${this.url}/pl_${this.termInfo.postingsFileName}`)).arrayBuffer();
  }

  getIt(): PlIterator {
    return new PlIterator(
      new DataView(this.arrayBuffer),
      this.termInfo.postingsFileOffset,
      this.termInfo.docFreq,
    );
  }
}

export default PostingsList;
