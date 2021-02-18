class DocInfo {
  normalizationFactors: number[] = [0];

  constructor(
    public docId: number,
  ) {}

  addDocLen(fieldId: number, tfIdf: number): void {
    for (let i = this.normalizationFactors.length; i <= fieldId; i += 1) {
      this.normalizationFactors.push(0);
    }

    this.normalizationFactors[fieldId] += tfIdf * tfIdf;
  }

  getDumpString(): string {
    const buffer = [];
    for (let i = 1; i < this.normalizationFactors.length; i += 1) {
      this.normalizationFactors[i] = Math.sqrt(this.normalizationFactors[i]);
      buffer.push(this.normalizationFactors[i].toFixed(6));
    }

    return buffer.join(',');
  }
}

export default DocInfo;
