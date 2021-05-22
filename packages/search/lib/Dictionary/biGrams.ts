export default function getBiGrams(term: string): string[] {
  const biGrams = [];

  const end = term.length - 1;
  for (let i = 0; i < end; i += 1) {
    biGrams.push(term[i] + term[i + 1]);
  }

  return biGrams;
}
