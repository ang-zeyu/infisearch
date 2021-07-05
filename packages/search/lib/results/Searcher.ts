import PostingsListManager from '../PostingsList/PostingsListManager';
import Dictionary from '../Dictionary/Dictionary';
import Query from './Query';
import { FieldInfosRaw, FieldInfo } from './FieldInfo';
import DocInfo from './DocInfo';
import parseQuery, { QueryPart, QueryPartType } from '../parser/queryParser';
import preprocess from '../parser/queryPreprocessor';
import postprocess from '../parser/queryPostProcessor';

class Searcher {
  private dictionary: Dictionary;

  private postingsListManager: PostingsListManager;

  private docInfo: DocInfo;

  private fieldStoreBlockSize: number;

  private fieldInfos: FieldInfo[];

  private setupPromise: Promise<void>;

  private tokenizer: (string) => string[];

  private stopWords: Set<string>;

  constructor(
    private url: string,
    private setupDictionaryUrl: string,
  ) {
    this.setupPromise = this.setup();
  }

  setupDocInfo(numScoredFields: number) {
    this.docInfo = new DocInfo(this.url, numScoredFields);
  }

  async setupFieldInfo(): Promise<{ numWeightedFields: number }> {
    const json: FieldInfosRaw = await (await fetch(`${this.url}/fieldInfo.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })).json();

    this.fieldStoreBlockSize = json.field_store_block_size;

    this.fieldInfos = [];
    Object.entries(json.field_infos_map).forEach(([fieldName, fieldInfo]) => {
      fieldInfo.name = fieldName;
      this.fieldInfos.push(fieldInfo as FieldInfo);
    });
    this.fieldInfos.sort((a, b) => a.id - b.id);

    console.log(this.fieldInfos);

    return { numWeightedFields: json.num_scored_fields };
  }

  async setup() {
    let wasmModule: any = import('../../../librarian_common/pkg/index.js');

    const { numWeightedFields } = await this.setupFieldInfo();

    this.setupDocInfo(numWeightedFields);
    await this.docInfo.initialisedPromise;

    this.dictionary = new Dictionary(this.setupDictionaryUrl);
    await this.dictionary.setup(this.url, this.docInfo.numDocs);

    this.postingsListManager = new PostingsListManager(
      this.url,
      this.dictionary,
      this.fieldInfos,
      this.docInfo.numDocs,
    );

    wasmModule = await wasmModule;
    this.tokenizer = wasmModule.wasm_tokenize;
    this.stopWords = new Set(JSON.parse(wasmModule.get_stop_words()));
  }

  getAggregatedTerms(queryParts: QueryPart[], seen: Set<string>, result: string[]) {
    queryParts.forEach((queryPart) => {
      if (queryPart.terms) {
        if (queryPart.isStopWordRemoved) {
          result.push(queryPart.originalTerms[0]);
        }

        queryPart.terms.forEach((term) => {
          if (seen.has(term)) {
            return;
          }

          result.push(term);
        });
      } else if (queryPart.children) {
        this.getAggregatedTerms(queryPart.children, seen, result);
      }
    });
  }

  async getQuery(query): Promise<Query> {
    await this.setupPromise;

    // TODO tokenize by language
    const queryParts = parseQuery(query, this.tokenizer);
    // console.log(JSON.stringify(queryParts, null, 4));
    // const queryTerms: string[] = query.toLowerCase().split(/\s+/g);

    /* const queryVectors = queryParts
      .map((queryTerm, idx) => {
        this.dictionary.getTerms(queryTerm, idx === queryTerms.length - 1);
      })
      .filter((queryVec) => queryVec.getAllTerms().length);
    const aggregatedTerms = queryVectors.reduce((acc, queryVec) => acc.concat(queryVec.getAllTerms()), []);
    console.log(aggregatedTerms); */

    const isFreeTextQuery = queryParts.every((queryPart) => queryPart.type === QueryPartType.TERM);

    const preProcessedQueryParts = await preprocess(queryParts, isFreeTextQuery, this.stopWords, this.dictionary);
    console.log('preprocessed');
    console.log(JSON.stringify(preProcessedQueryParts, null, 4));

    const postingsLists = await this.postingsListManager.retrieveTopLevelPls(preProcessedQueryParts);
    console.log('processed');
    // console.log(postingsLists);

    const postProcessedQueryParts = await postprocess(queryParts, postingsLists, this.dictionary, this.url);

    const aggregatedTerms: string[] = [];
    this.getAggregatedTerms(queryParts, new Set<string>(), aggregatedTerms);

    return new Query(
      aggregatedTerms,
      postProcessedQueryParts,
      postingsLists,
      isFreeTextQuery,
      this.docInfo,
      this.fieldInfos,
      this.dictionary,
      this.url,
      this.fieldStoreBlockSize,
    );
  }
}

export default Searcher;
