import * as path from 'path';
import * as fs from 'fs-extra';

import Storage from './Storage';

class CombinedFileStorage extends Storage {
  private texts: { [docId: number]: string } = {};

  add(fieldName: string, docId: number, text: string): void {
    this.texts[docId] = this.texts[docId]
      ? `${this.texts[docId]} ${text}`
      : text;
  }

  dump(): void {
    const fullOutputFilePath = path.join(this.outputFolderPath, this.baseName);
    fs.writeFileSync(fullOutputFilePath, Object.values(this.texts).join('\n'));
  }
}

export default CombinedFileStorage;
