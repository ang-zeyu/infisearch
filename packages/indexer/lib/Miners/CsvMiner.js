"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
// eslint-disable-next-line import/no-extraneous-dependencies
const parse = require("csv-parse/lib/sync");
const Miner_1 = require("./Miner");
const Field_1 = require("./Fields/Field");
const TextStorage_1 = require("./Fields/TextStorage");
const JsonStorage_1 = require("./Fields/JsonStorage");
class CsvMiner extends Miner_1.default {
    constructor(outputFolderPath) {
        const headingBodyStorage = new JsonStorage_1.default(outputFolderPath, { baseName: 'text', n: 1 });
        super(outputFolderPath, [
            new Field_1.default('title', 0.2, new TextStorage_1.default(outputFolderPath, { baseName: 'title', n: 100 })),
            new Field_1.default('heading', 0.3, headingBodyStorage),
            new Field_1.default('body', 0.5, headingBodyStorage),
            new Field_1.default('headingLink', 0, headingBodyStorage),
            new Field_1.default('link', 0, new TextStorage_1.default(outputFolderPath, { baseName: 'link', n: 100 })),
        ]);
    }
    indexCsvDoc(link, csvRaw) {
        const records = parse(csvRaw, {
            columns: true,
        });
        records.forEach((record) => {
            const fields = [];
            fields.push({ fieldName: 'link', text: 'dummylink' });
            fields.push({ fieldName: 'title', text: record.title });
            fields.push({ fieldName: 'body', text: record.content });
            this.add(fields);
        });
    }
}
exports.default = CsvMiner;
//# sourceMappingURL=CsvMiner.js.map