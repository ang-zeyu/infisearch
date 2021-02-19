class Result {
  storages: {
    [storageBaseName: string]: any
  } = Object.create(null);

  constructor(
    public docId: number,
    public score: number,
  ) {}
}

export default Result;
