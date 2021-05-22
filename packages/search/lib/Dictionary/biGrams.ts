const BIGRAM_START_CHAR = '^';
const BIGRAM_END_CHAR = '$';

export default function getBiGrams(term: string): string[] {
  const biGrams = [];
  biGrams.push(BIGRAM_START_CHAR + term[0]);

  const end = term.length - 1;
  for (let i = 0; i < end; i += 1) {
    biGrams.push(term[i] + term[i + 1]);
  }

  biGrams.push(term[end] + BIGRAM_END_CHAR);

  return biGrams;
}
