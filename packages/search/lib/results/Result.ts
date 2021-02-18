class Result {
  fields: {
    [fieldName: string]: string
  };

  constructor(
    public docId: number,
    public score: number,
  ) {}
}

export default Result;
