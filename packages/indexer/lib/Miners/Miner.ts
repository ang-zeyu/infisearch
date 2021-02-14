import * as path from 'path';
import * as fs from 'fs-extra';
import Tokenizer from '../tokenizers/English';
import Dictionary from '../Dictionary/Dictionary';
import PostingsListManager from '../Postings/PostingsListManager';
import DocInfo from '../DocInfo/DocInfo';

const tokenizer = new Tokenizer();

abstract class Miner {
  outputFolder: string;

  lastDocId: number = 0;

  docInfos: {
    [docId: number]: DocInfo
  } = {};

  dictionary: Dictionary = new Dictionary();

  postingsListManager: PostingsListManager = new PostingsListManager();

  protected constructor(outputFolder: string) {
    this.outputFolder = outputFolder;
  }

  protected add(link: string, serp: string, fields: { [fieldName: string]: string[] }) {
    this.lastDocId += 1;

    this.docInfos[this.lastDocId] = new DocInfo(this.lastDocId, link, serp);

    let pos: number = -1;
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    Object.entries(fields).forEach(([fieldName, texts]) => {
      texts.forEach((text) => {
        pos += 1;

        const terms = tokenizer.tokenize(text);
        terms.forEach((term) => {
          pos += 1;
          if (term.length > 255) {
            return;
          }

          this.postingsListManager.addTerm(term, this.lastDocId, pos);
        });
      });
    });
  }

  dump(): void {
    this.postingsListManager.dump(this.dictionary, this.docInfos, this.outputFolder);
    this.dictionary.dump(this.outputFolder);
    this.dumpDocInfo();
  }

  private dumpDocInfo(): void {
    fs.ensureDirSync(path.join(this.outputFolder, 'serps'));
    const linkFullPath = path.join(this.outputFolder, 'docInfo.txt');

    const numDocs = Object.keys(this.docInfos).length;
    const buffer = [`${numDocs}`];
    Object.entries(this.docInfos).forEach(([docId, info]) => {
      buffer.push(String(Math.sqrt(info.normalizationFactor)));
      buffer.push(info.link);
      fs.writeFileSync(path.join(this.outputFolder, 'serps', `${docId}`), info.serp);
    });

    fs.writeFileSync(linkFullPath, buffer.join('\n'));
  }
}

export default Miner;
