"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const flatMap = require('lodash/flatMap');
const PUNCTUATION_FILTER = new RegExp('[\\[\\](){}&|\'"`<>#:;~_^=\\-‑+*/‘’“”，。《》…—‐•?!,.]', 'g');
const BOUNDARY_FILTER = new RegExp('(^\\W)|(\\W$)');
const WHITESPACE_SPLITTER = new RegExp('\\s+', 'g');
const SENTENCE_SPLITTER = new RegExp('[.?!](?=\\s)', 'g');
class English {
    // eslint-disable-next-line class-methods-use-this
    tokenize(text) {
        return flatMap(text.toLowerCase().split(SENTENCE_SPLITTER), (sent) => sent.split(WHITESPACE_SPLITTER)
            .map((term) => term.replace(PUNCTUATION_FILTER, '').replace(BOUNDARY_FILTER, ''))
            .filter((term) => term.length !== 0 && term.length <= 255));
    }
}
exports.default = English;
//# sourceMappingURL=English.js.map