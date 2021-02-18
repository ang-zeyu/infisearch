import * as path from 'path';
import * as fs from 'fs-extra';

import Storage from './Storage';

class SingleFileStorage extends Storage {
  private texts: { [docId: number]: string } = {};

  add(fieldName: string, docId: number, text: string): void {
    this.texts[docId] = this.texts[docId]
      ? `${this.texts[docId]} ${text}`
      : text;
  }

  dump(): void {
    const fullOutputFolderPath = path.join(this.outputFolderPath, this.baseName);
    fs.ensureDirSync(fullOutputFolderPath);
    Object.entries(this.texts).forEach(([docId, text]) => {
      const fullOutputFilePath = path.join(fullOutputFolderPath, docId);
      fs.writeFileSync(fullOutputFilePath, text);
    });
  }
}

export default SingleFileStorage;
