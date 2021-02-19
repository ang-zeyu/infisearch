import * as path from 'path';
import * as fs from 'fs-extra';

import Storage from './Storage';

class TextStorage extends Storage {
  private texts: string[] = [''];

  private numDocsPerFile: number;

  constructor(outputFolderPath: string, params: { baseName: string, n: number }) {
    super(outputFolderPath, params);
    this.numDocsPerFile = params.n;
  }

  add(fieldId: number, docId: number, text: string): void {
    for (let i = this.texts.length; i <= docId; i += 1) {
      this.texts.push('');
    }
    this.texts[docId] = this.texts[docId] ? `${this.texts[docId]} ${text}` : text;
  }

  dump(): void {
    const fullOutputFolderPath = path.join(this.outputFolderPath, this.params.baseName);
    fs.ensureDirSync(fullOutputFolderPath);
    for (let i = 1; i < this.texts.length; i += this.numDocsPerFile) {
      const buffer = [];
      const end = i + this.numDocsPerFile;
      for (let j = i; j < end; j += 1) {
        buffer.push(this.texts[j]);
      }

      const fullOutputFilePath = path.join(fullOutputFolderPath, String(i));
      fs.writeFileSync(fullOutputFilePath, buffer.join('\n'));
    }
  }
}

export default TextStorage;
