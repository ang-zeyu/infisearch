"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
class PostingsList {
    constructor() {
        this.positions = {};
        this.termFreqs = {};
    }
    add(docId, fieldId, pos) {
        if (!this.positions[docId]) {
            this.positions[docId] = {};
            this.termFreqs[docId] = {};
        }
        if (!this.positions[docId][fieldId]) {
            this.positions[docId][fieldId] = [];
            this.termFreqs[docId][fieldId] = 0;
        }
        this.positions[docId][fieldId].push(pos);
        this.termFreqs[docId][fieldId] += 1;
    }
    getDocFreq() {
        return Object.keys(this.positions).length;
    }
}
exports.default = PostingsList;
//# sourceMappingURL=PostingsList.js.map