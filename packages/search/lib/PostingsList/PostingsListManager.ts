import decodeVarInt from '../utils/varInt';
import Dictionary from '../Dictionary/Dictionary';

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

  getDocs(term): { docId: number, termFreq: number }[] {
    if (!this.postingsLists[term]) {
      return [];
    }

    const docs = [];

    const view = new DataView(this.postingsLists[term]);
    const info = this.dictionary.termInfo[term];

    const end = info.postingsFileOffset + info.postingsFileLength;
    for (let i = info.postingsFileOffset; i < end;) {
      const docId = view.getUint16(i, true);
      i += 2;
      const termFreq = view.getUint16(i, true);
      i += 2;

      for (let j = 0; j < termFreq; j += 1) {
        const { value, pos } = decodeVarInt(view, i);
        i = pos;
      }

      docs.push({ docId, termFreq });
    }

    return docs;
  }
}

export default PostingsListManager;
