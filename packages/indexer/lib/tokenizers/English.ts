import Tokenizer from './Tokenizer';

const PUNCTUATION_FILTER = new RegExp('[\\[\\](){}&|\'"`<>#:;~_^=\\-‑+*/‘’“”，。《》…—‐•?!,.]', 'g');

class English implements Tokenizer {
  // eslint-disable-next-line class-methods-use-this
  tokenize(text: string): string[] {
    return text.split(/\s+/g)
      .map((term) => term.replace(PUNCTUATION_FILTER, ''));
  }
}

export default English;
