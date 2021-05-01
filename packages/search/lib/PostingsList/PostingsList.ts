import decodeVarInt from '../utils/varInt';
import TermInfo from '../results/TermInfo';

class PostingsList {
  private arrayBuffer: ArrayBuffer;

  private resultStore: Map<number, { [fieldId: number]: number[] }> = new Map();

  constructor(
    public readonly term: string,
    private readonly url: string,
    private readonly termInfo: TermInfo,
  ) {}

  async fetch(): Promise<void> {
    this.arrayBuffer = await (await fetch(`${this.url}/pl_${this.termInfo.postingsFileName}`)).arrayBuffer();
  }

  async getDocs(): Promise<Map<number, { [fieldId: number]: number[] }>> {
    let currentFileOffset = this.termInfo.postingsFileOffset;
    let numDocsRead = 0;
    const view = new DataView(this.arrayBuffer);

    while (numDocsRead < this.termInfo.docFreq) {
      const { value: docId, newPos: posAfterDocId } = decodeVarInt(view, currentFileOffset);
      currentFileOffset = posAfterDocId;

      const fieldPositions: { [fieldId: number]: number[] } = {};
      this.resultStore.set(docId, fieldPositions);

      let isLast = 0;
      do {
        const nextInt = view.getUint8(currentFileOffset);
        currentFileOffset += 1;

        /* eslint-disable no-bitwise */
        const fieldId = nextInt & 0x7f;
        isLast = nextInt & 0x80;
        /* eslint-enable no-bitwise */

        const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(view, currentFileOffset);
        currentFileOffset = posAfterTermFreq;

        fieldPositions[fieldId] = [];
        let prevPos = 0;
        for (let i = 0; i < fieldTermFreq; i += 1) {
          const { value: posGap, newPos: posAfterPosGap } = decodeVarInt(view, currentFileOffset);
          currentFileOffset = posAfterPosGap;

          const currPos = prevPos + posGap;
          fieldPositions[fieldId].push(currPos);
          prevPos = currPos;
        }
      } while (!isLast);

      numDocsRead += 1;
    }

    return this.resultStore;
  }
}

export default PostingsList;
