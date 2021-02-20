import * as path from 'path';
import * as fs from 'fs-extra';

import PostingsList from './PostingsList';
import Dictionary from '../Dictionary/Dictionary';
import DictionaryEntry from '../Dictionary/DictionaryEntry';

import getVarInt from './VarInt';
import DocInfo from '../DocInfo/DocInfo';

const POSTINGS_LIST_BLOCK_SIZE_MAX = 20000; // 20kb

class PostingsListManager {
  private postingsLists: { [term: string]: PostingsList } = Object.create(null);

  addTerm(fieldId: number, term: string, docId: number, pos: number): void {
    if (!this.postingsLists[term]) {
      this.postingsLists[term] = new PostingsList();
    }

    this.postingsLists[term].add(docId, fieldId, pos);
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

      const docFreq = postingsList.getDocFreq();
      const idf = Math.log10(numDocs / docFreq);

      let postingsFileLength = 0;

      const sortedEntries = Object.entries(postingsList.positions)
        .sort(([, docFieldPos1], [, docFieldPos2]) => {
          const totalTermFreq1 = Object.values(docFieldPos1).reduce((acc, pos) => acc + pos.length, 0);
          const totalTermFreq2 = Object.values(docFieldPos2).reduce((acc, pos) => acc + pos.length, 0);

          return totalTermFreq2 - totalTermFreq1;
        });

      // eslint-disable-next-line @typescript-eslint/no-loop-func
      sortedEntries.forEach(([docId, docFieldPositions]) => {
        const docIdInt = Number(docId);

        const docIdGapVarInt = getVarInt(docIdInt);
        postingsFileLength += docIdGapVarInt.length;
        buffers.push(docIdGapVarInt);

        const lastFieldIdx = Object.keys(docFieldPositions).length - 1;

        Object.entries(docFieldPositions).forEach(([fieldId, positions], idx) => {
          const fieldIdInt = Number(fieldId);
          const fieldTermFreq = positions.length;

          const buffer = Buffer.allocUnsafe(1);
          // eslint-disable-next-line no-bitwise
          buffer.writeUInt8(idx === lastFieldIdx ? (fieldIdInt | 0x80) : fieldIdInt);
          buffers.push(buffer);
          postingsFileLength += 1;

          const fieldTermFreqVarInt = getVarInt(fieldTermFreq);
          postingsFileLength += fieldTermFreqVarInt.length;
          buffers.push(fieldTermFreqVarInt);

          let prevPos = 0;
          positions.forEach((pos) => {
            const gap = getVarInt(pos - prevPos);
            prevPos = pos;

            postingsFileLength += gap.length;
            buffers.push(gap);
          });

          const wtd = 1 + Math.log10(fieldTermFreq);
          const tfIdf = wtd * idf;
          docInfos[docIdInt].addDocLen(fieldIdInt, tfIdf);
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
