"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
class PostingsList {
    constructor() {
        this.positions = {};
    }
    add(docId, fieldId, pos) {
        if (!this.positions[docId]) {
            this.positions[docId] = {};
        }
        if (!this.positions[docId][fieldId]) {
            this.positions[docId][fieldId] = [];
        }
        this.positions[docId][fieldId].push(pos);
    }
    getDocFreq() {
        return Object.keys(this.positions).length;
    }
}
exports.default = PostingsList;
//# sourceMappingURL=PostingsList.js.map