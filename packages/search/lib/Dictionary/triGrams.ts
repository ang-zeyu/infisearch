export default function getTriGrams(term: string): string[] {
  const triGrams = [];

  const end = term.length - 2;
  for (let i = 0; i < end; i += 1) {
    triGrams.push(term[i] + term[i + 1] + term[i + 2]);
  }

  return triGrams;
}
