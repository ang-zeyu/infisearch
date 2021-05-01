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
        let currentPl = 1;
        let currentPlOffset = 0;
        let currentPlBuffers = [];
        function moveToNextPl() {
            const postingsListFilePath = path.join(outputFolderPath, `pl_${currentPl}`);
            fs.writeFileSync(postingsListFilePath, Buffer.concat(currentPlBuffers));
            currentPl += 1;
            currentPlOffset = 0;
            currentPlBuffers = [];
        }
        for (let i = 0; i < sortedTerms.length; i += 1) {
            const currTerm = sortedTerms[i];
            const postingsList = this.postingsLists[currTerm];
            const docFreq = postingsList.getDocFreq();
            dictionary.entries[currTerm] = new DictionaryEntry_1.default(docFreq, currentPl, currentPlOffset);
            // Dump postings list
            // eslint-disable-next-line @typescript-eslint/no-loop-func
            Object.entries(postingsList.positions).forEach(([docId, docFieldPositions]) => {
                const docIdInt = Number(docId);
                const docIdVarInt = VarInt_1.default(docIdInt);
                currentPlOffset += docIdVarInt.length;
                currentPlBuffers.push(docIdVarInt);
                const lastFieldIdx = Object.keys(docFieldPositions).length - 1;
                Object.entries(docFieldPositions).forEach(([fieldId, positions], idx) => {
                    const fieldIdInt = Number(fieldId);
                    const fieldTermFreq = positions.length;
                    const buffer = Buffer.allocUnsafe(1);
                    // eslint-disable-next-line no-bitwise
                    buffer.writeUInt8(idx === lastFieldIdx ? (fieldIdInt | 0x80) : fieldIdInt);
                    currentPlBuffers.push(buffer);
                    currentPlOffset += 1;
                    const fieldTermFreqVarInt = VarInt_1.default(fieldTermFreq);
                    currentPlOffset += fieldTermFreqVarInt.length;
                    currentPlBuffers.push(fieldTermFreqVarInt);
                    let prevPos = 0;
                    for (let j = 0; j < positions.length; j += 1) {
                        const posGapVarInt = VarInt_1.default(positions[j] - prevPos);
                        currentPlOffset += posGapVarInt.length;
                        currentPlBuffers.push(posGapVarInt);
                        prevPos = positions[j];
                    }
                });
            });
            if (i === (sortedTerms.length - 1) || currentPlOffset > POSTINGS_LIST_BLOCK_SIZE_MAX) {
                moveToNextPl();
            }
        }
    }
}
exports.default = PostingsListManager;
//# sourceMappingURL=PostingsListManager.js.map