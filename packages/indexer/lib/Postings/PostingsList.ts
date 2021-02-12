class PostingsList {
  positions: { [docId: number]: number[] } = {};

  termFreqs: { [docId: number]: number } = {};

  add(docId: number, pos: number) {
    if (!this.positions[docId]) {
      this.positions[docId] = [];
      this.termFreqs[docId] = 0;
    }

    this.positions[docId].push(pos);
    this.termFreqs[docId] += 1;
  }
}

export default PostingsList;
