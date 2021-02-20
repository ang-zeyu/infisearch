import decodeVarInt from '../utils/varInt';
import Dictionary from '../Dictionary/Dictionary';
import PostingsList from './PostingsList';

class PostingsListManager {
  private postingsLists: {
    [term: string]: ArrayBuffer
  } = Object.create(null);

  constructor(
    private url: string,
    private dictionary: Dictionary,
  ) {}

  async retrieve(terms): Promise<void> {
    await Promise.all(terms
      .filter((term) => this.dictionary.termInfo[term] && !this.postingsLists[term])
      .map(async (term) => {
        if (this.postingsLists[term]) {
          return;
        }

        const info = this.dictionary.termInfo[term];
        this.postingsLists[term] = await (
          await fetch(`${this.url}/pl_${info.postingsFileName}`)
        ).arrayBuffer();
      }));
  }

  getDocs(term): PostingsList {
    if (!this.postingsLists[term]) {
      return new PostingsList();
    }

    const view = new DataView(this.postingsLists[term]);
    const info = this.dictionary.termInfo[term];
    const postingsList = new PostingsList();

    const end = info.postingsFileOffset + info.postingsFileLength;
    for (let i = info.postingsFileOffset; i < end;) {
      const { value: docId, newPos: posAfterDocId } = decodeVarInt(view, i);
      i = posAfterDocId;

      let isLast = 0;
      do {
        const nextInt = view.getUint8(i);
        i += 1;

        /* eslint-disable no-bitwise */
        const fieldId = nextInt & 0x7f;
        isLast = nextInt & 0x80;
        /* eslint-enable no-bitwise */

        const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(view, i);
        i = posAfterTermFreq;

        let posSoFar = 0;
        for (let j = 0; j < fieldTermFreq; j += 1) {
          const { value: posGap, newPos: posAfterPosGap } = decodeVarInt(view, i);
          i = posAfterPosGap;
          posSoFar += posGap;

          postingsList.add(docId, fieldId, posSoFar);
        }
      } while (!isLast);
    }

    return postingsList;
  }
}

export default PostingsListManager;
