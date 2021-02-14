import * as path from 'path';
import * as fs from 'fs-extra';

import PostingsList from './PostingsList';
import Dictionary from '../Dictionary/Dictionary';
import DictionaryEntry from '../Dictionary/DictionaryEntry';

import VarInt from './VarInt';
import DocInfo from '../DocInfo/DocInfo';

const POSTINGS_LIST_BLOCK_SIZE_MAX = 20000; // 20kb

class PostingsListManager {
  private postingsLists: { [term: string]: PostingsList } = Object.create(null);

  addTerm(term: string, docId: number, pos: number): void {
    if (!this.postingsLists[term]) {
      this.postingsLists[term] = new PostingsList();
    }

    this.postingsLists[term].add(docId, pos);
  }

  dump(dictionary: Dictionary, docInfos: { [docId: number]: DocInfo }, outputFolderPath: string): void {
    const numDocs = Object.keys(docInfos).length;
    const sortedTerms = Object.keys(this.postingsLists).sort();

    let postingsFileOffset = 0;
    let currentName = 1;
    let buffers = [];
    for (let i = 0; i < sortedTerms.length; i += 1) {
      const currTerm = sortedTerms[i];
      const postingsList = this.postingsLists[currTerm];

      const docFreq = Object.keys(postingsList.positions).length;
      const idf = Math.log10(numDocs / docFreq);

      let postingsFileLength = 0;
      // eslint-disable-next-line @typescript-eslint/no-loop-func
      Object.entries(postingsList.positions).forEach(([docId, positions]) => {
        const buffer = Buffer.allocUnsafe(4);

        const docIdInt = parseInt(docId, 10);
        buffer.writeUInt16LE(docIdInt);
        const termFreq = postingsList.termFreqs[docIdInt];
        buffer.writeUInt16LE(termFreq, 2);

        postingsFileLength += 4;
        buffers.push(buffer);

        const wtd = 1 + Math.log10(termFreq);
        const tfIdf = wtd * idf;
        docInfos[docId].normalizationFactor += tfIdf * tfIdf;

        let prevPos = 0;
        positions.forEach((pos) => {
          const gap = new VarInt(pos - prevPos);
          prevPos = pos;

          postingsFileLength += gap.value.length;
          buffers.push(gap.value);
        });
      });

      dictionary.entries[currTerm] = new DictionaryEntry(
        currTerm, docFreq, currentName,
        postingsFileOffset, postingsFileLength,
      );

      postingsFileOffset += postingsFileLength;

      if (i === (sortedTerms.length - 1) || postingsFileOffset > POSTINGS_LIST_BLOCK_SIZE_MAX) {
        const postingsListFilePath = path.join(outputFolderPath, `pl_${currentName}`);
        fs.writeFileSync(postingsListFilePath, Buffer.concat(buffers));

        postingsFileOffset = 0;
        currentName += 1;
        buffers = [];
      }
    }
  }
}

export default PostingsListManager;
