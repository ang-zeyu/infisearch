import decodeVarInt from '../utils/varInt';
import TermInfo from '../results/TermInfo';

class PostingsList {
  private arrayBuffer: ArrayBuffer;

  private resultStore: Map<number, { [fieldId: number]: number[] }> = new Map<number, { [p: number]: number[] }>();

  private currentOffset: number;

  private endOffset: number;

  constructor(
    public readonly term: string,
    private readonly url: string,
    private readonly termInfo: TermInfo,
  ) {
    this.currentOffset = termInfo.postingsFileOffset;
  }

  async fetch(): Promise<void> {
    this.arrayBuffer = await (await fetch(`${this.url}/pl_${this.termInfo.postingsFileName}`)).arrayBuffer();
    this.endOffset = this.termInfo.postingsFileEndName === this.termInfo.postingsFileName
      ? Math.min(this.termInfo.postingsFileEndOffset, this.arrayBuffer.byteLength)
      : this.arrayBuffer.byteLength;
  }

  getDocs(r: number): Map<number, { [fieldId: number]: number[] }> {
    const view = new DataView(this.arrayBuffer);

    let numDocsRead = this.resultStore.size;

    while (this.currentOffset < this.endOffset) {
      if (numDocsRead > r) {
        break;
      }
      numDocsRead += 1;

      const { value: docId, newPos: posAfterDocId } = decodeVarInt(view, this.currentOffset);
      this.currentOffset = posAfterDocId;

      const fieldPositions = {};
      this.resultStore.set(docId, fieldPositions);

      let isLast = 0;
      do {
        const nextInt = view.getUint8(this.currentOffset);
        this.currentOffset += 1;

        /* eslint-disable no-bitwise */
        const fieldId = nextInt & 0x7f;
        isLast = nextInt & 0x80;
        /* eslint-enable no-bitwise */

        fieldPositions[fieldId] = [];

        const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(view, this.currentOffset);
        this.currentOffset = posAfterTermFreq;

        let posSoFar = 0;
        for (let j = 0; j < fieldTermFreq; j += 1) {
          const { value: posGap, newPos: posAfterPosGap } = decodeVarInt(view, this.currentOffset);
          this.currentOffset = posAfterPosGap;
          posSoFar += posGap;

          fieldPositions[fieldId].push(posSoFar);
        }
      } while (!isLast);
    }

    return this.resultStore;
  }

  deleteDocs(docIds: number[]) {
    docIds.forEach((docId) => this.resultStore.delete(docId));
  }
}

export default PostingsList;
