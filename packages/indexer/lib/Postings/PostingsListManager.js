"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const PostingsList_1 = require("./PostingsList");
const DictionaryEntry_1 = require("../Dictionary/DictionaryEntry");
const VarInt_1 = require("./VarInt");
const POSTINGS_LIST_BLOCK_SIZE_MAX = 20000; // 20kb
class PostingsListManager {
    constructor(fieldInfo) {
        this.fieldInfo = fieldInfo;
        this.postingsLists = Object.create(null);
    }
    addTerm(fieldName, term, docId, pos) {
        if (!this.postingsLists[term]) {
            this.postingsLists[term] = new PostingsList_1.default();
        }
        this.postingsLists[term].add(this.fieldInfo[fieldName].id, docId, pos);
    }
    dump(dictionary, docInfos, outputFolderPath) {
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
            Object.entries(postingsList.positions).forEach(([docId, fields]) => {
                let totalTermFreq = 0;
                const docIdInt = Number(docId);
                Object.entries(this.fieldInfo).forEach(([fieldName, info]) => {
                    const fieldId = info.id;
                    const positions = fields[fieldId];
                    if (!positions) {
                        return;
                    }
                    const buffer = Buffer.allocUnsafe(5);
                    buffer.writeUInt16LE(docIdInt);
                    const fieldIdInt = Number(fieldId);
                    buffer.writeUInt8(fieldIdInt, 2);
                    const termFreq = postingsList.termFreqs[docIdInt][fieldIdInt];
                    buffer.writeUInt16LE(termFreq, 3);
                    postingsFileLength += 5;
                    buffers.push(buffer);
                    totalTermFreq += termFreq * this.fieldInfo[fieldName].weight;
                    let prevPos = 0;
                    positions.forEach((pos) => {
                        const gap = new VarInt_1.default(pos - prevPos);
                        prevPos = pos;
                        postingsFileLength += gap.value.length;
                        buffers.push(gap.value);
                    });
                });
                const wtd = 1 + Math.log10(totalTermFreq);
                const tfIdf = wtd * idf;
                docInfos[docId].normalizationFactor += tfIdf * tfIdf;
            });
            dictionary.entries[currTerm] = new DictionaryEntry_1.default(currTerm, docFreq, currentName, postingsFileOffset, postingsFileLength);
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
exports.default = PostingsListManager;
//# sourceMappingURL=PostingsListManager.js.map