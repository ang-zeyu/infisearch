"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
class DocInfo {
    constructor(docId) {
        this.docId = docId;
        this.normalizationFactors = [0];
    }
    addDocLen(fieldId, tfIdf) {
        for (let i = this.normalizationFactors.length; i <= fieldId; i += 1) {
            this.normalizationFactors.push(0);
        }
        this.normalizationFactors[fieldId] += tfIdf * tfIdf;
    }
    sqrtNormalizationFactors() {
        for (let i = 1; i < this.normalizationFactors.length; i += 1) {
            this.normalizationFactors[i] = Math.sqrt(this.normalizationFactors[i]);
        }
    }
    getDumpString() {
        const buffer = [];
        for (let i = 1; i < this.normalizationFactors.length; i += 1) {
            buffer.push(this.normalizationFactors[i].toFixed(6));
        }
        return buffer.join(',');
    }
}
exports.default = DocInfo;
//# sourceMappingURL=DocInfo.js.map