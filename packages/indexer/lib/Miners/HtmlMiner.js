"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const cheerio = require("cheerio");
const Miner_1 = require("./Miner");
const Field_1 = require("./Fields/Field");
const CombinedFileStorage_1 = require("./Fields/CombinedFileStorage");
const SingleFileStorage_1 = require("./Fields/SingleFileStorage");
const WHITESPACE = new RegExp('\\s+', 'g');
const blockHtmlElements = [
    'address',
    'article',
    'aside',
    'blockquote',
    'figcaption',
    'figure',
    'footer',
    'form',
    'h1',
    'h2',
    'h3',
    'h4',
    'h5',
    'h6',
    'header',
    'li',
    'main',
    'nav',
    'p',
    'div',
    'section',
    'td',
    'title',
];
const blockHtmlElementsSet = new Set(blockHtmlElements);
class HtmlMiner extends Miner_1.default {
    constructor(outputFolderPath) {
        super(outputFolderPath, {
            title: new Field_1.default('title', 1.5, new CombinedFileStorage_1.default(outputFolderPath, 'title')),
            heading: new Field_1.default('heading', 1.2, new CombinedFileStorage_1.default(outputFolderPath, 'heading')),
            body: new Field_1.default('body', 1, new SingleFileStorage_1.default(outputFolderPath, 'body')),
            link: new Field_1.default('link', 0, new CombinedFileStorage_1.default(outputFolderPath, 'link')),
        });
    }
    indexEl($, el, fields) {
        $(el).children().each((i, child) => {
            this.indexEl($, child, fields);
        });
        if (!blockHtmlElementsSet.has(el.name)) {
            return;
        }
        let fieldName;
        switch (el.name) {
            case 'title':
                fieldName = 'title';
                break;
            case 'h1':
            case 'h2':
            case 'h3':
            case 'h4':
            case 'h5':
            case 'h6':
                fieldName = 'heading';
                break;
            default:
                fieldName = 'body';
        }
        const elTxt = $(el).text().replace(WHITESPACE, ' ');
        $(el).text('');
        fields.push({ fieldName, text: elTxt });
    }
    indexHtmlDoc(link, htmlSource) {
        const fields = [];
        fields.push({ fieldName: 'link', text: link });
        fields.push({ fieldName: 'title', text: '' });
        fields.push({ fieldName: 'heading', text: '' });
        const $ = cheerio.load(htmlSource);
        this.indexEl($, $('html')[0], fields);
        this.add(fields);
    }
}
exports.default = HtmlMiner;
//# sourceMappingURL=HtmlMiner.js.map