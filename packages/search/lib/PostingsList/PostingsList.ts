// eslint-disable-next-line max-classes-per-file
import decodeVarInt from '../utils/varInt';
import TermInfo from '../results/TermInfo';

export interface DocField {
  fieldId: number,
  fieldPositions: number[]
}

export interface TermDoc {
  docId: number,
  fields: DocField[]
}

export class PlIterator {
  td: TermDoc;

  private idx = 0;

  constructor(public readonly pl: PostingsList) {
    // eslint-disable-next-line prefer-destructuring
    this.td = pl.termDocs[0];
  }

  next(): TermDoc {
    // eslint-disable-next-line no-return-assign, no-plusplus
    return this.td = this.pl.termDocs[++this.idx];
  }

  peekPrev(): TermDoc {
    return this.pl.termDocs[this.idx - 1];
  }
}

export class PostingsList {
  termDocs: TermDoc[] = [];

  weight = 1;

  includeInProximityRanking = true;

  termInfo: TermInfo;

  constructor(termInfo?: TermInfo) {
    this.termInfo = termInfo || ({} as any);
  }

  getIt(): PlIterator {
    return new PlIterator(this);
  }

  // Used for "processed" (e.g. phrase, bracket, AND) postings lists
  calcPseudoIdf(numDocs: number): void {
    this.termInfo.idf = Math.log(1 + (numDocs - this.termDocs.length + 0.5) / (this.termDocs.length + 0.5));
  }

  static mergeTermDocs(termDoc1: TermDoc, termDoc2: TermDoc): TermDoc {
    const td: TermDoc = {
      docId: termDoc1.docId,
      fields: [],
    };
    const maxFieldsLength = Math.max(termDoc1.fields.length, termDoc2.fields.length);
    for (let fieldId = 0; fieldId < maxFieldsLength; fieldId += 1) {
      const termDoc1Field = termDoc1.fields[fieldId];
      const termDoc2Field = termDoc2.fields[fieldId];

      if (termDoc1Field && termDoc2Field) {
        const docField: DocField = {
          fieldId,
          fieldPositions: [],
        };

        let pos2Idx = 0;
        for (let pos1Idx = 0; pos1Idx < termDoc1Field.fieldPositions.length; pos1Idx += 1) {
          while (termDoc2Field.fieldPositions[pos2Idx]
            && termDoc2Field.fieldPositions[pos2Idx] < termDoc1Field.fieldPositions[pos1Idx]
          ) {
            docField.fieldPositions.push(termDoc2Field.fieldPositions[pos2Idx]);
            pos2Idx += 1;
          }
          docField.fieldPositions.push(termDoc1Field.fieldPositions[pos1Idx]);
        }

        while (termDoc2Field.fieldPositions[pos2Idx]) {
          docField.fieldPositions.push(termDoc2Field.fieldPositions[pos2Idx]);
          pos2Idx += 1;
        }

        td.fields.push(docField);
      } else if (termDoc1Field) {
        td.fields.push({
          fieldId,
          fieldPositions: termDoc1Field.fieldPositions,
        });
      } else if (termDoc2Field) {
        td.fields.push({
          fieldId,
          fieldPositions: termDoc2Field.fieldPositions,
        });
      }
    }

    return td;
  }

  merge(other: PostingsList): PostingsList {
    const newPl = new PostingsList();

    let otherTermDocIdx = 0;
    for (const currTermDoc of this.termDocs) {
      while (
        otherTermDocIdx < other.termDocs.length
        && currTermDoc.docId > other.termDocs[otherTermDocIdx].docId
      ) {
        newPl.termDocs.push(other.termDocs[otherTermDocIdx]);
        otherTermDocIdx += 1;
      }

      if (
        otherTermDocIdx >= other.termDocs.length
        || currTermDoc.docId < other.termDocs[otherTermDocIdx].docId
      ) {
        newPl.termDocs.push(currTermDoc);
      } else if (other.termDocs[otherTermDocIdx].docId === currTermDoc.docId) {
        newPl.termDocs.push(PostingsList.mergeTermDocs(currTermDoc, other.termDocs[otherTermDocIdx]));
        otherTermDocIdx += 1;
      }
    }

    return newPl;
  }
}

export class TermPostingsList extends PostingsList {
  constructor(
    public readonly term: string,
    termInfo: TermInfo,
  ) {
    super(termInfo);
  }

  async fetch(baseUrl: string): Promise<void> {
    if (!this.termInfo) {
      return;
    }
    const arrayBuffer = await (await fetch(`${baseUrl}/pl_${this.termInfo.postingsFileName}`)).arrayBuffer();
    const dataView = new DataView(arrayBuffer);

    let prevDocId = 0;
    let pos = this.termInfo.postingsFileOffset;
    for (let i = 0; i < this.termInfo.docFreq; i += 1) {
      const { value: docIdGap, newPos: posAfterDocId } = decodeVarInt(dataView, pos);
      pos = posAfterDocId;

      const termDoc: TermDoc = {
        docId: prevDocId + docIdGap,
        fields: [],
      };
      prevDocId = termDoc.docId;

      let isLast = 0;
      do {
        const nextInt = dataView.getUint8(pos);
        pos += 1;

        /* eslint-disable no-bitwise */
        const fieldId = nextInt & 0x7f;
        isLast = nextInt & 0x80;
        /* eslint-enable no-bitwise */

        const { value: fieldTermFreq, newPos: posAfterTermFreq } = decodeVarInt(dataView, pos);
        pos = posAfterTermFreq;

        const fieldPositions = [];
        let prevPos = 0;
        for (let j = 0; j < fieldTermFreq; j += 1) {
          const { value: posGap, newPos: posAfterPosGap } = decodeVarInt(dataView, pos);
          pos = posAfterPosGap;

          const currPos = prevPos + posGap;
          fieldPositions.push(currPos);
          prevPos = currPos;
        }

        termDoc.fields[fieldId] = { fieldId, fieldPositions };
      } while (!isLast);

      this.termDocs.push(termDoc);
    }
  }
}
