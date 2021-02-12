"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const PUNCTUATION_FILTER = new RegExp('[\\[\\](){}&|\'"`<>#:;~_^=\\-‑+*/‘’“”，。《》…—‐•?!,.]', 'g');
class English {
    // eslint-disable-next-line class-methods-use-this
    tokenize(text) {
        return text.split(/\s+/g)
            .map((term) => term.replace(PUNCTUATION_FILTER, ''));
    }
}
exports.default = English;
//# sourceMappingURL=English.js.map