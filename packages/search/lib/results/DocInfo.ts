export default class DocInfo {
  public readonly initialisedPromise: Promise<void>;

  public readonly avgDocLengths: number[] = [];

  public readonly docLengthFactors: number[][] = [];

  public numDocs: number;

  constructor(url: string, numFields: number) {
    this.initialisedPromise = fetch(`${url}/docInfo`)
      .then((res) => res.arrayBuffer())
      .then((arrayBuffer) => {
        let byteOffset = 0;
        const view = new DataView(arrayBuffer);
        this.numDocs = view.getUint32(0, true);
        byteOffset += 4;

        for (let i = 0; i < numFields; i += 1) {
          this.avgDocLengths[i] = view.getUint32(byteOffset, true);
          byteOffset += 4;
        }

        while (byteOffset < arrayBuffer.byteLength) {
          const docFieldLengths = [];
          for (let i = 0; i < numFields; i += 1) {
            docFieldLengths.push(view.getUint32(byteOffset, true) / this.avgDocLengths[i]);
            byteOffset += 4;
          }
          this.docLengthFactors.push(docFieldLengths);
        }
      });
  }
}
