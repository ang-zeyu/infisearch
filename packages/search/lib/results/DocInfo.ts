export default class DocInfo {
  public readonly initialisedPromise: Promise<void>;

  public readonly docLengths: number[][] = [];

  public numDocs: number;

  constructor(url: string, numFields: number) {
    this.initialisedPromise = fetch(`${url}/docInfo`)
      .then((res) => res.arrayBuffer())
      .then((arrayBuffer) => {
        const view = new DataView(arrayBuffer);
        this.numDocs = view.getUint32(0, true);
        for (let byteOffset = 4; byteOffset < arrayBuffer.byteLength; byteOffset += numFields * 8) {
          const docFieldLengths = [];
          for (let j = 0; j < numFields; j += 1) {
            docFieldLengths.push(view.getFloat64(byteOffset, true));
          }
          this.docLengths.push(docFieldLengths);
        }
      });
  }
}
