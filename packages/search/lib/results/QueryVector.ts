class QueryVector {
  public readonly mainTermAndWeight: { [term: string]: number } = Object.create(null);

  public readonly correctedTermsAndWeights: { [term: string]: number } = Object.create(null);

  public readonly expandedTermsAndWeights: { [term: string]: number } = Object.create(null);

  setTerm(term: string, weight: number): void {
    this.mainTermAndWeight[term] = weight;
  }

  addCorrectedTerm(term: string, weight: number): void {
    this.correctedTermsAndWeights[term] = weight;
  }

  addExpandedTerm(term: string, weight: number): void {
    this.expandedTermsAndWeights[term] = weight;
  }

  getAllTerms(): string[] {
    return [
      ...Object.keys(this.mainTermAndWeight),
      ...Object.keys(this.correctedTermsAndWeights),
      ...Object.keys(this.expandedTermsAndWeights),
    ];
  }

  getAllTermsAndWeights(): { [term: string]: number } {
    return Object.assign(
      Object.assign(
        Object.assign(Object.create(null), this.expandedTermsAndWeights),
        this.correctedTermsAndWeights,
      ),
      this.mainTermAndWeight,
    );
  }

  getTermWeight(term: string) {
    return this.mainTermAndWeight[term]
      ?? this.correctedTermsAndWeights[term]
      ?? this.expandedTermsAndWeights[term];
  }

  toString(): string {
    return this.getAllTerms()
      .reduce((acc, term) => `${acc} | ${term} ${this.getTermWeight(term)}`, '');
  }
}

export default QueryVector;
