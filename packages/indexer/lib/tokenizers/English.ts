import Tokenizer from './Tokenizer';

const flatMap = require('lodash/flatMap');

const PUNCTUATION_FILTER = new RegExp('[\\[\\](){}&|\'"`<>#:;~_^=\\-‑+*/‘’“”，。《》…—‐•?!,.]', 'g');

const WHITESPACE_SPLITTER = new RegExp('\\s+', 'g');
const SENTENCE_SPLITTER = new RegExp('[.?!](?=\\s)', 'g');

class English implements Tokenizer {
  // eslint-disable-next-line class-methods-use-this
  tokenize(text: string): string[] {
    return flatMap(text.toLowerCase().split(SENTENCE_SPLITTER), (sent) => sent.split(WHITESPACE_SPLITTER)
      .map((term) => term.replace(PUNCTUATION_FILTER, ''))
      .filter((term) => term.length !== 0 && term.length <= 255));
  }
}

export default English;
