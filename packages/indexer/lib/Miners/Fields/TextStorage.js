"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const Storage_1 = require("./Storage");
class TextStorage extends Storage_1.default {
    constructor(outputFolderPath, params) {
        super(outputFolderPath, params);
        this.texts = [''];
        this.numDocsPerFile = params.n;
    }
    add(fieldName, docId, text) {
        for (let i = this.texts.length; i <= docId; i += 1) {
            this.texts.push('');
        }
        this.texts[docId] = `${this.texts[docId]} ${text}`;
    }
    dump() {
        const fullOutputFolderPath = path.join(this.outputFolderPath, this.params.baseName);
        fs.ensureDirSync(fullOutputFolderPath);
        for (let i = 1; i < this.texts.length; i += this.numDocsPerFile) {
            const buffer = [];
            const end = i + this.numDocsPerFile;
            for (let j = i; j < end; j += 1) {
                buffer.push(this.texts[j]);
            }
            const fullOutputFilePath = path.join(fullOutputFolderPath, String(i));
            fs.writeFileSync(fullOutputFilePath, buffer.join('\n'));
        }
    }
}
exports.default = TextStorage;
//# sourceMappingURL=TextStorage.js.map