"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const Storage_1 = require("./Storage");
class CombinedFileStorage extends Storage_1.default {
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
        const fullOutputFilePath = path.join(this.outputFolderPath, this.baseName);
        fs.writeFileSync(fullOutputFilePath, Object.values(this.texts).join('\n'));
    }
}
exports.default = CombinedFileStorage;
//# sourceMappingURL=CombinedFileStorage.js.map