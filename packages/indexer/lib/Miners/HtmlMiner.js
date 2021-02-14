"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const cheerio = require("cheerio");
const Miner_1 = require("./Miner");
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
];
const blockHtmlElementsSet = new Set(blockHtmlElements);
class HtmlMiner extends Miner_1.default {
    // eslint-disable-next-line @typescript-eslint/no-useless-constructor
    constructor(outputFolderPath) {
        super(outputFolderPath);
    }
    indexEl($, el, fields) {
        var _a;
        $(el).children().each((i, child) => {
            this.indexEl($, child, fields);
        });
        if (!blockHtmlElementsSet.has(el.name)) {
            return;
        }
        fields[el.name] = (_a = fields[el.name]) !== null && _a !== void 0 ? _a : [];
        const elTxt = $(el).text().toLowerCase();
        $(el).text('');
        fields[el.name].push(elTxt);
    }
    indexHtmlDoc(link, htmlSource) {
        const $ = cheerio.load(htmlSource);
        const serp = $.root().text().replace(WHITESPACE, ' ');
        const fields = Object.create(null);
        this.indexEl($, $('html')[0], fields);
        this.add(link, serp, fields);
    }
}
exports.default = HtmlMiner;
//# sourceMappingURL=HtmlMiner.js.map