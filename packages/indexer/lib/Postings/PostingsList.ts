class PostingsList {
  positions: {
    [docId: number]: {
      [fieldId: number]: number[]
    }
  } = {};

  add(docId: number, fieldId: number, pos: number) {
    if (!this.positions[docId]) {
      this.positions[docId] = {};
    }

    if (!this.positions[docId][fieldId]) {
      this.positions[docId][fieldId] = [];
    }

    this.positions[docId][fieldId].push(pos);
  }

  getDocFreq() {
    return Object.keys(this.positions).length;
  }
}

export default PostingsList;
