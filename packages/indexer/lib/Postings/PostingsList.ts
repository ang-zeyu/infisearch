class PostingsList {
  positions: {
    [docId: number]: {
      [fieldId: number]: number[]
    }
  } = {};

  termFreqs: {
    [docId: number]: {
      [fieldId: number]: number
    }
  } = {};

  add(docId: number, fieldId: number, pos: number) {
    if (!this.positions[docId]) {
      this.positions[docId] = {};
      this.termFreqs[docId] = {};
    }

    if (!this.positions[docId][fieldId]) {
      this.positions[docId][fieldId] = [];
      this.termFreqs[docId][fieldId] = 0;
    }

    this.positions[docId][fieldId].push(pos);
    this.termFreqs[docId][fieldId] += 1;
  }

  getDocFreq() {
    return Object.keys(this.positions).length;
  }
}

export default PostingsList;
