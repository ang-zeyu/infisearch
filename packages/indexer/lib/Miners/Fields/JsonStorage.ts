import * as path from 'path';
import * as fs from 'fs-extra';

import Storage from './Storage';

class JsonStorage extends Storage {
  private texts: (string | number)[][] = [];

  private lastFieldId: { [docId: number]: number } = {};

  private numDocsPerFile: number;

  constructor(outputFolderPath: string, params: { baseName: string, n: number }) {
    super(outputFolderPath, params);
    this.numDocsPerFile = params.n;
  }

  add(fieldId: number, docId: number, text: string): void {
    const end = docId - 1;
    for (let i = this.texts.length; i <= end; i += 1) {
      this.texts.push([]);
    }

    if (this.lastFieldId[docId] !== fieldId) {
      this.texts[end].push(fieldId);
      this.lastFieldId[docId] = fieldId;
    }

    this.texts[end].push(text);
  }

  dump(): void {
    const fullOutputFolderPath = path.join(this.outputFolderPath, this.params.baseName);
    fs.ensureDirSync(fullOutputFolderPath);
    for (let i = 0; i < this.texts.length; i += this.numDocsPerFile) {
      const slice: (string | number)[][] = [];

      const end = i + this.numDocsPerFile;
      for (let j = i; j < end; j += 1) {
        slice.push(this.texts[j]);
      }

      const fullOutputFilePath = path.join(fullOutputFolderPath, `${i}.json`);
      fs.writeJSONSync(fullOutputFilePath, slice);
    }
  }
}

export default JsonStorage;
