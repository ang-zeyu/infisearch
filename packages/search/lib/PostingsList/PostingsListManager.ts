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
      const docId = view.getUint16(i, true);
      i += 2;
      const fieldId = view.getUint8(i);
      i += 1;
      const termFreq = view.getUint16(i, true);
      i += 2;

      for (let j = 0; j < termFreq; j += 1) {
        const { value, pos } = decodeVarInt(view, i);
        i = pos;

        postingsList.add(docId, fieldId, value);
      }
    }

    return postingsList;
  }
}

export default PostingsListManager;
