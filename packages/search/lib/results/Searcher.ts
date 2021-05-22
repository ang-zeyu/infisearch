import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Query from './Query';
import FieldInfo from './FieldInfo';
import DocInfo from './DocInfo';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docInfo: DocInfo;

  private fieldInfo: FieldInfo;

  private setupPromise: Promise<void>;

  constructor(
    private url: string,
    private setupDictionaryUrl: string,
  ) {
    this.setupPromise = this.setup();
  }

  setupDocInfo(fieldInfoJson: any) {
    this.docInfo = new DocInfo(this.url,
      Object.values(fieldInfoJson).filter((field: any) => field.weight !== 0).length);
  }

  async setup() {
    const json = await (await fetch(`${this.url}/fieldInfo.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    this.setupDocInfo(json);
    await this.docInfo.initialisedPromise;

    this.dictionary = new Dictionary();
    await this.dictionary.setup(this.setupDictionaryUrl, this.url, this.docInfo.numDocs);

    this.postingsListManager = new PostingsListManager(this.url, this.dictionary);

    Object.keys(json).forEach((fieldName) => {
      json[json[fieldName].id] = json[fieldName];
      json[fieldName].name = fieldName;
    });

    this.fieldInfo = json;
    console.log(this.fieldInfo);
  }

  async getQuery(query): Promise<Query> {
    await this.setupPromise;

    // TODO tokenize by language
    const queryTerms: string[] = query.split(/\s+/g);

    const queryVectors = queryTerms
      .map((queryTerm, idx) => this.dictionary.getTerms(queryTerm, idx === queryTerms.length - 1))
      .filter((queryVec) => queryVec.getAllTerms().length);
    const aggregatedTerms = queryVectors.reduce((acc, queryVec) => acc.concat(queryVec.getAllTerms()), []);
    console.log(aggregatedTerms);

    const postingsLists = await this.postingsListManager.retrieve(aggregatedTerms);

    return new Query(
      aggregatedTerms, queryVectors, this.docInfo, this.fieldInfo, this.dictionary, this.url, postingsLists,
    );
  }
}

export default Searcher;
