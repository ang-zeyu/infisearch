"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
class PostingsList {
    constructor() {
        this.positions = {};
        this.termFreqs = {};
    }
    add(docId, pos) {
        if (!this.positions[docId]) {
            this.positions[docId] = [];
            this.termFreqs[docId] = 0;
        }
        this.positions[docId].push(pos);
        this.termFreqs[docId] += 1;
    }
}
exports.default = PostingsList;
//# sourceMappingURL=PostingsList.js.map