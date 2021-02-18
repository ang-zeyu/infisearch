"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const varInt_1 = require("../Postings/varInt");
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
        const buffers = [];
        const sortedTerms = Object.keys(this.entries).sort();
        let prevPostingsFileName = 0;
        for (let i = 0; i < sortedTerms.length; i += 1) {
            const entry = this.entries[sortedTerms[i]];
            const postingsFileNameBuffer = Buffer.allocUnsafe(1);
            postingsFileNameBuffer.writeUInt8(entry.postingsFileName - prevPostingsFileName);
            prevPostingsFileName = entry.postingsFileName;
            buffers.push(postingsFileNameBuffer);
            buffers.push(varInt_1.default(entry.docFreq));
            buffers.push(varInt_1.default(entry.postingsFileLength));
            buffers.push(varInt_1.default(entry.postingsFileOffset));
        }
        const fullPath = path.join(folderPath, 'dictionaryTable.txt');
        fs.writeFileSync(fullPath, Buffer.concat(buffers));
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
                if (commonPrefixLen <= 2) {
                    break;
                }
                if (commonPrefixLen < currCommonPrefix.length) {
                    if (commonPrefixLen === currCommonPrefix.length - 1) {
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
            const termBuffer = Buffer.from(sortedTerms[i]);
            const currPrefixBuffer = Buffer.from(currCommonPrefix);
            buffers.push(Buffer.from([termBuffer.length]));
            buffers.push(currPrefixBuffer);
            if (numFrontcodedTerms > 0) {
                buffers.push(Buffer.from(`*${sortedTerms[i].substring(currCommonPrefix.length)}`));
            }
            while (numFrontcodedTerms > 0) {
                i += 1;
                numFrontcodedTerms -= 1;
                const frontCodedTermBuffer = Buffer.from(sortedTerms[i]);
                buffers.push(Buffer.from([frontCodedTermBuffer.length - currPrefixBuffer.length]));
                buffers.push(Buffer.from(`&${sortedTerms[i].substring(currCommonPrefix.length)}`));
            }
        }
        fs.writeFileSync(fullPath, Buffer.concat(buffers));
    }
}
exports.default = Dictionary;
//# sourceMappingURL=Dictionary.js.map