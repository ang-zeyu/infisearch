import decodeVarInt from '../utils/varInt';
import TermInfo from '../results/TermInfo';

class PostingsList {
  private arrayBuffer: ArrayBuffer;

  private resultStore: Map<number, { [fieldId: number]: number[] }> = new Map<number, { [p: number]: number[] }>();

  private currentFileName: number;

  private currentFileOffset: number;

  private currentFileEndOffset: number;

  constructor(
    public readonly term: string,
    private readonly url: string,
    private readonly termInfo: TermInfo,
  ) {
    this.currentFileName = termInfo.postingsFileName;
  }

  async fetch(): Promise<void> {
    console.log(`Fetching new pl_${this.currentFileName} for ${this.term}, last at ${this.termInfo.postingsFileEndName} ${this.termInfo.postingsFileEndOffset}`);

    this.arrayBuffer = await (await fetch(`${this.url}/pl_${this.currentFileName}`)).arrayBuffer();

    this.currentFileOffset = this.currentFileName === this.termInfo.postingsFileName
      ? this.termInfo.postingsFileOffset
      : 0;

    this.currentFileEndOffset = this.currentFileName !== this.termInfo.postingsFileEndName
      ? this.arrayBuffer.byteLength
      // Math.min to account for the last term, where postingsFileEndOffset is set to = Number.MAX_VALUE
      : Math.min(this.termInfo.postingsFileEndOffset, this.arrayBuffer.byteLength);
  }

  async getDocs(r: number): Promise<Map<number, { [fieldId: number]: number[] }>> {
    while (this.resultStore.size < r) {
      this.readRDocsFromCurrentFile(r - this.resultStore.size);
      if (this.resultStore.size < r) {
        const isLastFile = this.currentFileName === this.termInfo.postingsFileEndName
          || (this.currentFileName === this.termInfo.postingsFileEndName - 1
            && this.termInfo.postingsFileEndOffset === 0);
        if (isLastFile) {
          break;
        }

        this.currentFileName += 1;
        // eslint-disable-next-line no-await-in-loop
        await this.fetch();
      }
    }

    return this.resultStore;
  }

  private readRDocsFromCurrentFile(r: number): number {
    const view = new DataView(this.arrayBuffer);

    let numDocsRead = 0;
    while (this.currentFileOffset < this.currentFileEndOffset) {
      if (numDocsRead >= r) {
        return numDocsRead;
      }
      numDocsRead += 1;

      const { value: docId, newPos: posAfterDocId } = decodeVarInt(view, this.currentFileOffset);
      this.currentFileOffset = posAfterDocId;

      const fieldPositions = {};
      this.resultStore.set(docId, fieldPositions);

      let isLast = 0;
      do {
        const nextInt = view.getUint8(this.currentFileOffset);
        this.currentFileOffset += 1;

        /* eslint-disable no-bitwise */
        const fieldId = nextInt & 0x7f;
        isLast = nextInt & 0x80;
        /* eslint-enable no-bitwise */

        fieldPositions[fieldId] = [];

        const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(view, this.currentFileOffset);
        this.currentFileOffset = posAfterTermFreq;

        let posSoFar = 0;
        for (let j = 0; j < fieldTermFreq; j += 1) {
          const { value: posGap, newPos: posAfterPosGap } = decodeVarInt(view, this.currentFileOffset);
          this.currentFileOffset = posAfterPosGap;
          posSoFar += posGap;

          fieldPositions[fieldId].push(posSoFar);
        }
      } while (!isLast);
    }

    return numDocsRead;
  }

  deleteDocs(docIds: number[]) {
    docIds.forEach((docId) => this.resultStore.delete(docId));
  }
}

export default PostingsList;
