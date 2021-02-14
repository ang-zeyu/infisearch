class DocInfo {
  normalizationFactor: number = 0;

  constructor(
    public docId: number,
    public link: string,
    public serp: string,
  ) {
  }
}

export default DocInfo;
