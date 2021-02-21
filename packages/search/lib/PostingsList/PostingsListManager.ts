import Dictionary from '../Dictionary/Dictionary';
import PostingsList from './PostingsList';

class PostingsListManager {
  constructor(
    private url: string,
    private dictionary: Dictionary,
  ) {}

  retrieve(terms: string[]): Promise<PostingsList[]> {
    return Promise.all(terms
      .map(async (term) => {
        const postingsList = new PostingsList(term, this.url, this.dictionary.termInfo[term]);
        await postingsList.fetch();

        return postingsList;
      }));
  }
}

export default PostingsListManager;
