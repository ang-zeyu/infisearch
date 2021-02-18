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

  constructor(
    private fieldInfo: {
      [fieldName: string]: {
        id: number,
        storage: string,
        storageParams: { [param: string]: any },
        weight: number
      }
    },
  ) {}

  addTerm(fieldName: string, term: string, docId: number, pos: number): void {
    if (!this.postingsLists[term]) {
      this.postingsLists[term] = new PostingsList();
    }

    this.postingsLists[term].add(docId, this.fieldInfo[fieldName].id, pos);
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
      // eslint-disable-next-line @typescript-eslint/no-loop-func
      Object.entries(postingsList.termFreqs).forEach(([docId, fields]) => {
        let totalTermFreq = 0;

        const docIdInt = Number(docId);

        Object.entries(this.fieldInfo).forEach(([fieldName, info]) => {
          const fieldId = info.id;
          const fieldTermFreq = fields[fieldId];
          if (!fieldTermFreq) {
            return;
          }

          const buffer = Buffer.allocUnsafe(5);
          buffer.writeUInt16LE(docIdInt);
          buffer.writeUInt8(fieldId, 2);
          buffer.writeUInt16LE(fieldTermFreq, 3);
          buffers.push(buffer);
          postingsFileLength += 5;

          totalTermFreq += fieldTermFreq * this.fieldInfo[fieldName].weight;

          let prevPos = 0;
          postingsList.positions[docIdInt][fieldId].forEach((pos) => {
            const gap = new VarInt(pos - prevPos);
            prevPos = pos;

            postingsFileLength += gap.value.length;
            buffers.push(gap.value);
          });
        });

        if (totalTermFreq > 0) {
          const wtd = 1 + Math.log10(totalTermFreq);
          const tfIdf = wtd * idf;
          docInfos[docIdInt].normalizationFactor += tfIdf * tfIdf;
        }
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
