"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
class Dictionary {
    constructor() {
        this.entries = Object.create(null);
    }
    static getCommonPrefixLength(str1, str2) {
        let len = 0;
        while (len < str1.length && len < str2.length
            && str1.charAt(len) === str2.charAt(len)) {
            len += 1;
        }
        return len;
    }
    dump(folderPath) {
        this.dumpDictAsAString(folderPath);
        this.dumpDictTable(folderPath);
    }
    dumpDictTable(folderPath) {
        const fullPath = path.join(folderPath, 'dictionaryTable.txt');
        const buffer = Buffer.allocUnsafe(16);
        const sortedTerms = Object.keys(this.entries).sort();
        for (let i = 0; i < sortedTerms.length; i += 1) {
            const entry = this.entries[sortedTerms[i]];
            buffer.writeUInt32LE(entry.postingsFileName);
            buffer.writeUInt32LE(entry.docFreq, 4);
            buffer.writeUInt32LE(entry.postingsFileLength, 8);
            buffer.writeUInt32LE(entry.postingsFileOffset, 12);
        }
        fs.writeFileSync(fullPath, buffer);
    }
    dumpDictAsAString(folderPath) {
        const fullPath = path.join(folderPath, 'dictionaryString.txt');
        const buffers = [];
        const sortedTerms = Object.keys(this.entries).sort();
        for (let i = 0; i < sortedTerms.length; i += 1) {
            let currCommonPrefix = sortedTerms[i];
            let numFrontcodedTerms = 0;
            let j = i + 1;
            while (j < sortedTerms.length) {
                const commonPrefixLen = Dictionary.getCommonPrefixLength(currCommonPrefix, sortedTerms[j]);
                if (commonPrefixLen === 0) {
                    break;
                }
                if (commonPrefixLen < currCommonPrefix.length) {
                    const totalDiff = (currCommonPrefix.length - commonPrefixLen) * numFrontcodedTerms;
                    if (totalDiff === numFrontcodedTerms) {
                        // equally worth it
                        currCommonPrefix = currCommonPrefix.substring(0, commonPrefixLen);
                    }
                    else {
                        // not worth it
                        break;
                    }
                }
                numFrontcodedTerms += 1;
                j += 1;
            }
            buffers.push(Buffer.from([sortedTerms[i].length]));
            buffers.push(Buffer.from(currCommonPrefix));
            if (numFrontcodedTerms > 0) {
                buffers.push(Buffer.from(`*${sortedTerms[i].substring(currCommonPrefix.length)}`));
            }
            while (numFrontcodedTerms > 0) {
                i += 1;
                numFrontcodedTerms -= 1;
                buffers.push(Buffer.from([sortedTerms[i].length - currCommonPrefix.length]));
                buffers.push(Buffer.from(`&${sortedTerms[i].substring(currCommonPrefix.length)}`));
            }
        }
        fs.writeFileSync(fullPath, Buffer.concat(buffers));
    }
}
exports.default = Dictionary;
//# sourceMappingURL=Dictionary.js.map