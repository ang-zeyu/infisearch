import * as path from 'path';
import * as fs from 'fs-extra';

import PostingsList from './PostingsList';
import Dictionary from '../Dictionary/Dictionary';
import DictionaryEntry from '../Dictionary/DictionaryEntry';

import VarInt from './VarInt';

const POSTINGS_LIST_BLOCK_SIZE_MAX = 20000; // 20kb

class PostingsListManager {
  private postingsLists: { [term: string]: PostingsList } = Object.create(null);

  addTerm(term: string, docId: number, pos: number): void {
    if (!this.postingsLists[term]) {
      this.postingsLists[term] = new PostingsList();
    }

    this.postingsLists[term].add(docId, pos);
  }

  dump(dictionary: Dictionary, outputFolderPath: string): void {
    const sortedTerms = Object.keys(this.postingsLists).sort();

    let currentOffsetTotal = 0;
    let currentBufferLength = 0;
    let currentName = 1;
    let buffers = [];
    for (let i = 0; i < sortedTerms.length; i += 1) {
      const currTerm = sortedTerms[i];
      const postingsList = this.postingsLists[currTerm];
      const postingsFileOffset = currentBufferLength + currentOffsetTotal;

      let postingsFileLength = 4;
      Object.entries(postingsList.positions).forEach(([docId, positions]) => {
        const buffer = Buffer.allocUnsafe(4);

        const docIdInt = parseInt(docId, 10);
        buffer.writeInt16LE(docIdInt);

        const termFreq = postingsList.termFreqs[docIdInt];
        buffer.writeInt16LE(termFreq, 2);

        buffers.push(buffer);

        const prevPos = 0;
        positions.forEach((pos) => {
          const gap = new VarInt(pos - prevPos);
          postingsFileLength += gap.value.length;
          buffers.push(gap.value);
        });
      });
      currentBufferLength += postingsFileLength;

      const docFreq = Object.keys(postingsList.positions).length;
      dictionary.entries[currTerm] = new DictionaryEntry(
        currTerm, docFreq, currentName,
        postingsFileOffset, postingsFileLength,
      );

      if (i === (sortedTerms.length - 1) || currentBufferLength > POSTINGS_LIST_BLOCK_SIZE_MAX) {
        const postingsListFilePath = path.join(outputFolderPath, `pl_${currentName}`);
        fs.writeFileSync(postingsListFilePath, Buffer.concat(buffers));

        currentOffsetTotal += currentBufferLength;
        currentBufferLength = 0;
        currentName += 1;
        buffers = [];
      }
    }
  }
}

export default PostingsListManager;
