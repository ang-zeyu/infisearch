"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const Storage_1 = require("./Storage");
class SingleFileStorage extends Storage_1.default {
    constructor() {
        super(...arguments);
        this.texts = {};
    }
    add(fieldName, docId, text) {
        this.texts[docId] = this.texts[docId]
            ? `${this.texts[docId]} ${text}`
            : text;
    }
    dump() {
        const fullOutputFolderPath = path.join(this.outputFolderPath, this.baseName);
        fs.ensureDirSync(fullOutputFolderPath);
        Object.entries(this.texts).forEach(([docId, text]) => {
            const fullOutputFilePath = path.join(fullOutputFolderPath, docId);
            fs.writeFileSync(fullOutputFilePath, text);
        });
    }
}
exports.default = SingleFileStorage;
//# sourceMappingURL=SingleFileStorage.js.map