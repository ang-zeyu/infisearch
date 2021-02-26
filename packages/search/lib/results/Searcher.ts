import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Query from './Query';
import Storage from './Storage';
import FieldInfo from './FieldInfo';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docLengths: Promise<number[][]>;

  private fieldInfo: Promise<FieldInfo>;

  private storages: {
    [baseName: string]: Storage
  } = Object.create(null);

  constructor(
    private url: string,
  ) {
    this.dictionary = new Dictionary(url);
    this.postingsListManager = new PostingsListManager(url, this.dictionary);
    this.docLengths = this.setupDocLengths();
    this.fieldInfo = this.setupFieldInfo();
  }

  async setupDocLengths(): Promise<number[][]> {
    const text = await (await fetch(`${this.url}/docInfo.txt`, {
      method: 'GET',
      headers: {
        'Content-Type': 'text/plain',
      },
    })).text();

    const docLengths = [];
    text.split('\n').forEach((line) => {
      docLengths.push(line.split(',').map(parseFloat));
    });

    return docLengths;
  }

  private setupStorage(fieldInfo: FieldInfo) {
    Object.values(fieldInfo).forEach((field) => {
      if (this.storages[field.storageParams.baseName]) {
        return;
      }

      this.storages[field.storageParams.baseName] = new Storage(
        field.storage, field.storageParams, this.url, fieldInfo,
      );
    });
  }

  async setupFieldInfo() {
    const json = await (await fetch(`${this.url}/fieldInfo.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    Object.keys(json).forEach((fieldName) => {
      json[json[fieldName].id] = json[fieldName];
      json[json[fieldName].id].name = fieldName;
      delete json[json[fieldName].id].id;
      delete json[fieldName];
    });
    console.log(json);

    this.setupStorage(json);

    return json;
  }

  async getQuery(query): Promise<Query> {
    await this.dictionary.setupPromise;

    const queryTerms: string[] = query.split(/\s+/g);
    const queryVectors = queryTerms.map((queryTerm, idx) => this.dictionary.getTerms(queryTerm,
      idx === queryTerms.length - 1));
    const aggregatedTerms = queryVectors.reduce((acc, queryVec) => acc.concat(queryVec.getTerms()), []);

    const postingsLists = await this.postingsListManager.retrieve(aggregatedTerms);

    const docLengths = await this.docLengths;
    const fieldInfo = await this.fieldInfo;

    return new Query(aggregatedTerms, queryVectors, this.storages, docLengths, fieldInfo, this.dictionary, postingsLists);
  }
}

export default Searcher;
