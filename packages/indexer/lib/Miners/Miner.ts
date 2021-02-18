import * as path from 'path';
import * as fs from 'fs-extra';
import Tokenizer from '../tokenizers/English';
import Dictionary from '../Dictionary/Dictionary';
import PostingsListManager from '../Postings/PostingsListManager';
import DocInfo from '../DocInfo/DocInfo';
import Field from './Fields/Field';

const tokenizer = new Tokenizer();

abstract class Miner {
  lastDocId: number = 0;

  docInfos: {
    [docId: number]: DocInfo
  } = {};

  fieldInfo: {
    [fieldName: string]: {
      id: number,
      storage: string,
      baseFileName: string,
      weight: number
    }
  } = Object.create(null);

  dictionary: Dictionary = new Dictionary();

  postingsListManager: PostingsListManager;

  protected constructor(
    private outputFolder: string,
    private fields: { [fieldName: string]: Field },
  ) {
    let fieldId = 0;
    Object.values(fields).forEach((field) => {
      fieldId += 1;
      this.fieldInfo[field.name] = {
        id: fieldId,
        storage: field.storage.constructor.name,
        baseFileName: field.storage.baseName,
        weight: field.weight,
      };
    });
    this.postingsListManager = new PostingsListManager(this.fieldInfo);
  }

  protected add(fields: { fieldName: string, text: string }[]) {
    this.lastDocId += 1;

    this.docInfos[this.lastDocId] = new DocInfo(this.lastDocId);

    let pos = -1;
    fields.forEach((item) => {
      const { fieldName, text } = item;

      const field = this.fields[fieldName];
      if (!field) {
        throw new Error('Indexing undefined field.');
      }

      pos += 1;

      field.add(this.lastDocId, text);
      if (!field.weight) {
        // E.g. auxillary document info - links
        return;
      }

      const terms = tokenizer.tokenize(text);
      terms.forEach((term) => {
        pos += 1;
        if (term.length > 255) {
          return;
        }

        this.postingsListManager.addTerm(field.name, term, this.lastDocId, pos);
      });
    });
  }

  dump(): void {
    this.postingsListManager.dump(this.dictionary, this.docInfos, this.outputFolder);
    this.dictionary.dump(this.outputFolder);
    this.dumpDocInfo();
    this.dumpFields();
  }

  private dumpDocInfo(): void {
    const linkFullPath = path.join(this.outputFolder, 'docInfo.txt');

    const numDocs = Object.keys(this.docInfos).length;
    const buffer = [`${numDocs}`];
    Object.values(this.docInfos).forEach((info) => {
      buffer.push(String(Math.sqrt(info.normalizationFactor)));
    });

    fs.writeFileSync(linkFullPath, buffer.join('\n'));
  }

  private dumpFields(): void {
    fs.writeJSONSync(path.join(this.outputFolder, 'fieldInfo.json'), this.fieldInfo);
    Object.values(this.fields).forEach((field) => field.dump());
  }
}

export default Miner;
