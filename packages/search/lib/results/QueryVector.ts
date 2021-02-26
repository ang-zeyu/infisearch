class QueryVector {
  public readonly termsAndWeights: { [term: string]: number } = Object.create(null);

  addTerm(term: string, weight: number): void {
    this.termsAndWeights[term] = weight;
  }

  getTerms(): string[] {
    return Object.keys(this.termsAndWeights);
  }

  toString(): string {
    return Object.entries(this.termsAndWeights)
      .reduce((acc, [term, weight]) => `${acc} | ${term} ${weight}`, '');
  }
}

export default QueryVector;
