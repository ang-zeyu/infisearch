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

    let prevDocId = 0;
    const end = info.postingsFileOffset + info.postingsFileLength;
    for (let i = info.postingsFileOffset; i < end;) {
      const { value: docIdGap, newPos: posAfterDocId } = decodeVarInt(view, i);
      const docId = docIdGap + prevDocId;
      prevDocId = docId;
      i = posAfterDocId;

      const fieldId = view.getUint8(i);
      i += 1;

      const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(view, i);
      i = posAfterTermFreq;

      for (let j = 0; j < fieldTermFreq; j += 1) {
        const { value, newPos } = decodeVarInt(view, i);
        i = newPos;

        postingsList.add(docId, fieldId, value);
      }
    }

    return postingsList;
  }
}

export default PostingsListManager;
