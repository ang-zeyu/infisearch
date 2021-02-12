"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const PostingsList_1 = require("./PostingsList");
const DictionaryEntry_1 = require("../Dictionary/DictionaryEntry");
const VarInt_1 = require("./VarInt");
const POSTINGS_LIST_BLOCK_SIZE_MAX = 20000; // 20kb
class PostingsListManager {
    constructor() {
        this.postingsLists = Object.create(null);
    }
    addTerm(term, docId, pos) {
        if (!this.postingsLists[term]) {
            this.postingsLists[term] = new PostingsList_1.default();
        }
        this.postingsLists[term].add(docId, pos);
    }
    dump(dictionary, outputFolderPath) {
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
                    const gap = new VarInt_1.default(pos - prevPos);
                    postingsFileLength += gap.value.length;
                    buffers.push(gap.value);
                });
            });
            currentBufferLength += postingsFileLength;
            const docFreq = Object.keys(postingsList.positions).length;
            dictionary.entries[currTerm] = new DictionaryEntry_1.default(currTerm, docFreq, currentName, postingsFileOffset, postingsFileLength);
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
exports.default = PostingsListManager;
//# sourceMappingURL=PostingsListManager.js.map