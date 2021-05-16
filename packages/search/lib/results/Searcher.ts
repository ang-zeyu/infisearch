import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Query from './Query';
import FieldInfo from './FieldInfo';
import DocInfo from './DocInfo';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docInfo: DocInfo;

  private fieldInfo: Promise<FieldInfo>;

  constructor(
    private url: string,
  ) {
    this.dictionary = new Dictionary(url);
    this.postingsListManager = new PostingsListManager(url, this.dictionary);
    this.fieldInfo = this.setupFieldInfo();
  }

  setupDocInfo(fieldInfoJson: any) {
    this.docInfo = new DocInfo(this.url,
      Object.values(fieldInfoJson).filter((field: any) => field.weight !== 0).length);
  }

  async setupFieldInfo() {
    const json = await (await fetch(`${this.url}/fieldInfo.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    this.setupDocInfo(json);

    Object.keys(json).forEach((fieldName) => {
      json[json[fieldName].id] = json[fieldName];
      json[fieldName].name = fieldName;
    });
    console.log(json);

    return json;
  }

  async getQuery(query): Promise<Query> {
    await this.dictionary.setupPromise;

    // TODO tokenize by language
    const queryTerms: string[] = query.split(/\s+/g);

    const queryVectors = queryTerms
      .map((queryTerm, idx) => this.dictionary.getTerms(queryTerm, idx === queryTerms.length - 1))
      .filter((queryVec) => queryVec.getAllTerms().length);
    const aggregatedTerms = queryVectors.reduce((acc, queryVec) => acc.concat(queryVec.getAllTerms()), []);
    console.log(aggregatedTerms);

    const postingsLists = await this.postingsListManager.retrieve(aggregatedTerms);

    await this.docInfo.initialisedPromise;
    const fieldInfo = await this.fieldInfo;

    return new Query(
      aggregatedTerms, queryVectors, this.docInfo, fieldInfo, this.dictionary, this.url, postingsLists,
    );
  }
}

export default Searcher;
