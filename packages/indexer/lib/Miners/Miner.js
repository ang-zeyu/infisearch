"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const path = require("path");
const fs = require("fs-extra");
const English_1 = require("../tokenizers/English");
const Dictionary_1 = require("../Dictionary/Dictionary");
const PostingsListManager_1 = require("../Postings/PostingsListManager");
const tokenizer = new English_1.default();
class Miner {
    constructor(outputFolder) {
        this.lastDocId = 0;
        this.docInfos = {};
        this.dictionary = new Dictionary_1.default();
        this.postingsListManager = new PostingsListManager_1.default();
        this.outputFolder = outputFolder;
    }
    add(link, serp, fields) {
        this.lastDocId += 1;
        this.docInfos[this.lastDocId] = {
            link,
            serp,
        };
        let pos = -1;
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        Object.entries(fields).forEach(([fieldName, texts]) => {
            texts.forEach((text) => {
                pos += 1;
                const terms = tokenizer.tokenize(text);
                terms.forEach((term) => {
                    pos += 1;
                    if (term.length > 255) {
                        return;
                    }
                    this.postingsListManager.addTerm(term, this.lastDocId, pos);
                });
            });
        });
    }
    dump() {
        this.postingsListManager.dump(this.dictionary, this.outputFolder);
        this.dictionary.dump(this.outputFolder);
        this.dumpDocInfo();
    }
    dumpDocInfo() {
        fs.ensureDirSync(path.join(this.outputFolder, 'serps'));
        const linkFullPath = path.join(this.outputFolder, 'links.txt');
        const linksBuffer = [];
        Object.entries(this.docInfos).forEach(([docId, info]) => {
            linksBuffer.push(info.link);
            fs.writeFileSync(path.join(this.outputFolder, 'serps', `${docId}`), info.serp);
        });
        fs.writeFileSync(linkFullPath, linksBuffer.join('\n'));
    }
}
exports.default = Miner;
//# sourceMappingURL=Miner.js.map