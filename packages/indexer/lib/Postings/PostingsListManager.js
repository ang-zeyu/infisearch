"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const PostingsList_1 = require("./PostingsList");
const DictionaryEntry_1 = require("../Dictionary/DictionaryEntry");
const VarInt_1 = require("./VarInt");
const POSTINGS_LIST_BLOCK_SIZE_MAX = 65535; // Max that 2 bytes can store
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
    calcNormalizationFactors(sortedTerms, numDocs, docInfos) {
        for (let i = 0; i < sortedTerms.length; i += 1) {
            const postingsList = this.postingsLists[sortedTerms[i]];
            const docFreq = postingsList.getDocFreq();
            const idf = Math.log10(numDocs / docFreq);
            Object.entries(postingsList.positions).forEach(([docId, docFieldPositions]) => {
                const docIdInt = Number(docId);
                Object.entries(docFieldPositions).forEach(([fieldId, positions]) => {
                    const fieldIdInt = Number(fieldId);
                    const fieldTermFreq = positions.length;
                    const wtd = 1 + Math.log10(fieldTermFreq);
                    const tfIdf = wtd * idf;
                    docInfos[docIdInt].addDocLen(fieldIdInt, tfIdf);
                });
            });
        }
        Object.values(docInfos).forEach((docInfo) => docInfo.sqrtNormalizationFactors());
    }
    dump(dictionary, docInfos, outputFolderPath) {
        const numDocs = Object.keys(docInfos).length;
        const sortedTerms = Object.keys(this.postingsLists).sort();
        this.calcNormalizationFactors(sortedTerms, numDocs, docInfos);
        let postingsFileOffset = 0;
        let currentName = 1;
        let buffers = [];
        function moveToNextFile() {
            const postingsListFilePath = path.join(outputFolderPath, `pl_${currentName}`);
            fs.writeFileSync(postingsListFilePath, Buffer.concat(buffers));
            postingsFileOffset = 0;
            currentName += 1;
            buffers = [];
        }
        for (let i = 0; i < sortedTerms.length; i += 1) {
            const currTerm = sortedTerms[i];
            const postingsList = this.postingsLists[currTerm];
            const docFreq = postingsList.getDocFreq();
            dictionary.entries[currTerm] = new DictionaryEntry_1.default(currTerm, docFreq, currentName, postingsFileOffset);
            // Impact order the entries
            const sortedEntries = Object.entries(postingsList.positions)
                .map(([docId, docFieldPositions]) => {
                const docIdInt = Number(docId);
                let score = 0;
                Object.entries(docFieldPositions).forEach(([fieldId, positions]) => {
                    const fieldIdInt = Number(fieldId);
                    const fieldTermFreq = positions.length;
                    score += (fieldTermFreq / docInfos[docIdInt].normalizationFactors[fieldIdInt]) * this.fieldWeights[fieldIdInt];
                });
                return [docId, docFieldPositions, score];
            })
                .sort(([, , score1], [, , score2]) => score2 - score1);
            // Dump
            // eslint-disable-next-line @typescript-eslint/no-loop-func
            sortedEntries.forEach(([docId, docFieldPositions]) => {
                const docIdInt = Number(docId);
                const docIdGapVarInt = VarInt_1.default(docIdInt);
                postingsFileOffset += docIdGapVarInt.length;
                buffers.push(docIdGapVarInt);
                const lastFieldIdx = Object.keys(docFieldPositions).length - 1;
                Object.entries(docFieldPositions).forEach(([fieldId, positions], idx) => {
                    const fieldIdInt = Number(fieldId);
                    const fieldTermFreq = positions.length;
                    const buffer = Buffer.allocUnsafe(1);
                    // eslint-disable-next-line no-bitwise
                    buffer.writeUInt8(idx === lastFieldIdx ? (fieldIdInt | 0x80) : fieldIdInt);
                    buffers.push(buffer);
                    postingsFileOffset += 1;
                    const fieldTermFreqVarInt = VarInt_1.default(fieldTermFreq);
                    postingsFileOffset += fieldTermFreqVarInt.length;
                    buffers.push(fieldTermFreqVarInt);
                    let prevPos = 0;
                    positions.forEach((pos) => {
                        const gap = VarInt_1.default(pos - prevPos);
                        prevPos = pos;
                        postingsFileOffset += gap.length;
                        buffers.push(gap);
                    });
                });
                if (postingsFileOffset > POSTINGS_LIST_BLOCK_SIZE_MAX) {
                    moveToNextFile();
                }
            });
            if (i === (sortedTerms.length - 1)) {
                moveToNextFile();
            }
        }
    }
}
exports.default = PostingsListManager;
//# sourceMappingURL=PostingsListManager.js.map