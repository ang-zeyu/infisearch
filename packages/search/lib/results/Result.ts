class Result {
  fields: {
    [fieldName: string]: string
  } = Object.create(null);

  constructor(
    public docId: number,
    public score: number,
  ) {}
}

export default Result;
