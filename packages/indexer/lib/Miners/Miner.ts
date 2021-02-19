import * as path from 'path';
import * as fs from 'fs-extra';

import Tokenizer from '../tokenizers/English';
import Dictionary from '../Dictionary/Dictionary';
import PostingsListManager from '../Postings/PostingsListManager';
import DocInfo from '../DocInfo/DocInfo';
import Field from './Fields/Field';
import FieldInfo from './Fields/FieldInfo';

const clone = require('lodash/clone');

const tokenizer = new Tokenizer();

abstract class Miner {
  lastDocId: number = 0;

  docInfos: {
    [docId: number]: DocInfo
  } = {};

  fieldInfo: FieldInfo = Object.create(null);

  dictionary: Dictionary = new Dictionary();

  postingsListManager: PostingsListManager;

  private fields: { [fieldName: string]: Field } = Object.create(null);

  protected constructor(
    private outputFolder: string,
    fields: Field[],
  ) {
    let totalWeight = 0;
    let fieldId = 0;
    fields.forEach((field) => {
      fieldId += 1;
      field.id = fieldId;

      this.fieldInfo[field.name] = {
        id: fieldId,
        storage: field.storage.constructor.name,
        storageParams: field.storage.params,
        weight: field.weight,
      };

      this.fields[field.name] = field;

      totalWeight += field.weight;
    });

    if (totalWeight !== 1) {
      throw new Error('Field weights must sum to 1.');
    }

    this.postingsListManager = new PostingsListManager(this.fieldInfo);
  }

  protected add(fields: { fieldName: string, text: string }[]) {
    this.lastDocId += 1;

    this.docInfos[this.lastDocId] = new DocInfo(this.lastDocId);

    const uninitializedFields: { [fieldName: string]: Field } = clone(this.fields);

    let pos = -1;
    fields.forEach((item) => {
      const { fieldName, text } = item;

      pos += 1;

      delete uninitializedFields[fieldName];

      const field = this.fields[fieldName];
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

    Object.values(uninitializedFields).forEach((field) => field.add(this.lastDocId, ''));
  }

  dump(): void {
    this.postingsListManager.dump(this.dictionary, this.docInfos, this.outputFolder);
    this.dictionary.dump(this.outputFolder);
    this.dumpDocInfo();
    this.dumpFields();
  }

  private dumpDocInfo(): void {
    const numDocs = Object.keys(this.docInfos).length;
    const buffer = [`${numDocs}`, ...Object.values(this.docInfos).map((info) => info.getDumpString())];
    const linkFullPath = path.join(this.outputFolder, 'docInfo.txt');
    fs.writeFileSync(linkFullPath, buffer.join('\n'));
  }

  private dumpFields(): void {
    fs.writeJSONSync(path.join(this.outputFolder, 'fieldInfo.json'), this.fieldInfo);
    Object.values(this.fields).forEach((field) => field.dump());
  }
}

export default Miner;
