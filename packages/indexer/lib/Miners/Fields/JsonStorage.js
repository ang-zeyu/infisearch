"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const Storage_1 = require("./Storage");
class JsonStorage extends Storage_1.default {
    constructor(outputFolderPath, params) {
        super(outputFolderPath, params);
        this.texts = [];
        this.numDocsPerFile = params.n;
    }
    add(fieldName, docId, text) {
        const end = docId - 1;
        for (let i = this.texts.length; i <= end; i += 1) {
            this.texts.push([]);
        }
        this.texts[end].push(text);
    }
    dump() {
        const fullOutputFolderPath = path.join(this.outputFolderPath, this.params.baseName);
        fs.ensureDirSync(fullOutputFolderPath);
        for (let i = 0; i < this.texts.length; i += this.numDocsPerFile) {
            const slice = [];
            const end = i + this.numDocsPerFile;
            for (let j = i; j < end; j += 1) {
                slice.push(this.texts[j]);
            }
            const fullOutputFilePath = path.join(fullOutputFolderPath, `${i}.json`);
            fs.writeJSONSync(fullOutputFilePath, slice);
        }
    }
}
exports.default = JsonStorage;
//# sourceMappingURL=JsonStorage.js.map