"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
class DictionaryEntry {
    constructor(term, docFreq, postingsFileName, postingsFileOffset) {
        this.term = term;
        this.docFreq = docFreq;
        this.postingsFileName = postingsFileName;
        this.postingsFileOffset = postingsFileOffset;
    }
}
exports.default = DictionaryEntry;
//# sourceMappingURL=DictionaryEntry.js.map