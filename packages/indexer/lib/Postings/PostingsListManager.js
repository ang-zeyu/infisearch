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
        this.postingsLists = Object.create(null);
        this.fieldWeights = {};
        Object.values(fieldInfo).forEach((info) => {
            this.fieldWeights[info.id] = info.weight;
        });
    }
    addTerm(fieldId, term, docId, pos) {
        if (!this.postingsLists[term]) {
            this.postingsLists[term] = new PostingsList_1.default();
        }
        this.postingsLists[term].add(docId, fieldId, pos);
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
            // Calculate normalization factors and impact order the entries
            const sortedEntries = Object.entries(postingsList.positions)
                .map(([docId, docFieldPositions]) => {
                const docIdInt = Number(docId);
                let score = 0;
                Object.entries(docFieldPositions).forEach(([fieldId, positions]) => {
                    const fieldIdInt = Number(fieldId);
                    const fieldTermFreq = positions.length;
                    const wtd = 1 + Math.log10(fieldTermFreq);
                    const tfIdf = wtd * idf;
                    docInfos[docIdInt].addDocLen(fieldIdInt, tfIdf);
                    // doc length is constant for impact ordering a single posting list
                    score += (tfIdf * this.fieldWeights[fieldIdInt]);
                });
                return [docId, docFieldPositions, score];
            })
                .sort(([, , score1], [, , score2]) => score2 - score1);
            // Dump
            // eslint-disable-next-line @typescript-eslint/no-loop-func
            sortedEntries.forEach(([docId, docFieldPositions]) => {
                const docIdInt = Number(docId);
                const docIdGapVarInt = VarInt_1.default(docIdInt);
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
                    const fieldTermFreqVarInt = VarInt_1.default(fieldTermFreq);
                    postingsFileLength += fieldTermFreqVarInt.length;
                    buffers.push(fieldTermFreqVarInt);
                    let prevPos = 0;
                    positions.forEach((pos) => {
                        const gap = VarInt_1.default(pos - prevPos);
                        prevPos = pos;
                        postingsFileLength += gap.length;
                        buffers.push(gap);
                    });
                });
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