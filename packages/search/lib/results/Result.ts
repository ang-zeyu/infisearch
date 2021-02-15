class Result {
  heading: string;

  constructor(
    public docId: number,
    public score: number,
    public link: string,
  ) {
    this.heading = 'heading';
  }
}

export default Result;
