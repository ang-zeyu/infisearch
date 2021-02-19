"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
class Field {
    constructor(name, weight, storage) {
        this.name = name;
        this.weight = weight;
        this.storage = storage;
    }
    add(docId, text) {
        this.storage.add(this.id, docId, text);
    }
    dump() {
        this.storage.dump();
    }
}
exports.default = Field;
//# sourceMappingURL=Field.js.map