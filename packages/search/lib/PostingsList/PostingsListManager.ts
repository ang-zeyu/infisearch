import Dictionary from '../Dictionary/Dictionary';
import {
  DocField, PlIterator, PostingsList, TermDoc, TermPostingsList,
} from './PostingsList';
import { QueryPart, QueryPartType } from '../parser/queryParser';
import FieldInfo from '../results/FieldInfo';

const Heap = require('heap');

class PostingsListManager {
  constructor(
    private url: string,
    private dictionary: Dictionary,
    private fieldInfos: FieldInfo[],
    private numDocs: number,
  ) {}

  async populateTermPostingsLists(
    queryParts: QueryPart[],
    postingsLists: { [term: string]: TermPostingsList },
  ) {
    const promisesToAwait = [];
    for (const queryPart of queryParts) {
      if (queryPart.terms) {
        for (const term of queryPart.terms) {
          if (!postingsLists[term]) {
            postingsLists[term] = new TermPostingsList(term, {} as any);
            promisesToAwait.push(this.dictionary.getTermInfo(term).then((termInfo) => {
              postingsLists[term].termInfo = termInfo;
            }));
          }
        }
      } else if (queryPart.children) {
        promisesToAwait.push(this.populateTermPostingsLists(queryPart.children, postingsLists));
      }
    }
    await Promise.all(promisesToAwait);
  }

  populatePhrasePostingsList(queryPart: QueryPart, postingsLists: { [term: string]: TermPostingsList })
    : PostingsList {
    const pls = queryPart.terms.map((term) => postingsLists[term].getIt());
    const docHeap: Heap<PlIterator> = new Heap((a, b) => a.td.docId - b.td.docId);
    for (let i = 0; i < pls.length; i += 1) {
      docHeap.push(pls[i]);
    }

    const resultPl: PostingsList = new PostingsList();
    let currDocId = -1;
    let currNumDocs = 0;
    while (!docHeap.empty()) {
      const minPlIterator = docHeap.pop();

      if (minPlIterator.td.docId === currDocId) {
        currNumDocs += 1;

        if (minPlIterator.next()) {
          docHeap.push(minPlIterator);
        }

        if (currNumDocs === pls.length) {
          // The doc contains all the terms, now intersect positions
          const td: TermDoc = {
            docId: currDocId,
            fields: [],
          };
          let hasMatch = false;

          const termTermDocs = pls.map((plIt) => plIt.peekPrev());

          // Intersect all postings lists field by field
          for (let fieldId = 0; fieldId < this.fieldInfos.length; fieldId += 1) {
            const resultDocField: DocField = {
              fieldId,
              fieldPositions: [],
            };

            const termFieldPositionsIdxes: number[] = [];
            for (let termIdx = 0; termIdx < queryPart.terms.length; termIdx += 1) {
              termFieldPositionsIdxes[termIdx] = 0;
            }

            // Repeatedly go through postings lists in sequence
            let currPos = -10;
            for (let termIdx = 0; termIdx < queryPart.terms.length;) {
              const currPlField = termTermDocs[termIdx].fields[fieldId];
              if (!currPlField) {
                // field not present in current doc, then definitely we can't match anything here
                break;
              }

              const pos = currPlField.fieldPositions[termFieldPositionsIdxes[termIdx]];
              if (!pos) {
                // exceeded number of positions
                break;
              }

              termFieldPositionsIdxes[termIdx] += 1;
              if (pos === currPos + 1) {
                if (termIdx === queryPart.terms.length - 1) {
                  // Complete the match
                  hasMatch = true;
                  resultDocField.fieldPositions.push(pos - pls.length + 1);
                  currPos = -10;
                  termIdx = 0;
                } else {
                  // Match next term
                  currPos = pos;
                  termIdx += 1;
                }
              } else if (termIdx !== 0) {
                // Not matched

                // Forward this postings list up to currPos, try again
                if (pos < currPos) {
                  while (currPlField.fieldPositions[termFieldPositionsIdxes[termIdx]] < currPos) {
                    termFieldPositionsIdxes[termIdx] += 1;
                  }
                  continue;
                }

                // Reset
                currPos = -10;
                termIdx = 0;
              } else {
                currPos = pos;
                termIdx += 1;
              }
            }

            td.fields.push(resultDocField.fieldPositions.length ? resultDocField : undefined);
          }

          currDocId = -1;
          currNumDocs = 0;

          if (hasMatch) {
            resultPl.termDocs.push(td);
          }
        }
      } else {
        currDocId = minPlIterator.td.docId;
        currNumDocs = 1;

        if (minPlIterator.next()) {
          docHeap.push(minPlIterator);
        }
      }
    }

    resultPl.calcPseudoIdf(this.numDocs);

    return resultPl;
  }

  populateANDPostingsList(queryPart: QueryPart, postingsLists: { [term: string]: TermPostingsList })
    : PostingsList {
    const pls = this.populatePostingsLists(queryPart.children, postingsLists).map((pl) => pl.getIt());

    const docHeap: Heap<PlIterator> = new Heap((a, b) => a.td.docId - b.td.docId);
    for (const plIt of pls) {
      if (!plIt.td) {
        plIt.pl.calcPseudoIdf(this.numDocs);
        return plIt.pl; // intersection with empty postings list
      }
      docHeap.push(plIt);
    }

    const resultPl: PostingsList = new PostingsList();
    let currDocId = -1;
    let currNumDocs = 0;
    while (!docHeap.empty()) {
      const minPlIterator = docHeap.pop();

      if (minPlIterator.td.docId === currDocId) {
        if (minPlIterator.next()) {
          docHeap.push(minPlIterator);
        }

        currNumDocs += 1;

        if (currNumDocs === pls.length) {
          resultPl.termDocs.push(
            pls.map((pl) => pl.peekPrev())
              .reduce((acc, td2) => (acc ? PostingsList.mergeTermDocs(acc, td2) : td2)),
          );

          currDocId = -1;
          currNumDocs = 0;
        }
      } else {
        currDocId = minPlIterator.td.docId;
        currNumDocs = 1;

        if (minPlIterator.next()) {
          docHeap.push(minPlIterator);
        }
      }
    }

    resultPl.calcPseudoIdf(this.numDocs);

    return resultPl;
  }

  populatePostingsLists(queryParts: QueryPart[], postingsLists: { [term: string]: TermPostingsList })
    : PostingsList[] {
    return queryParts.map((queryPart) => {
      if (queryPart.type === QueryPartType.TERM) {
        return queryPart.terms[0] ? postingsLists[queryPart.terms[0]] : new PostingsList();
      } if (queryPart.type === QueryPartType.PHRASE) {
        return this.populatePhrasePostingsList(queryPart, postingsLists);
      } if (queryPart.type === QueryPartType.AND) {
        return this.populateANDPostingsList(queryPart, postingsLists);
      } if (queryPart.type === QueryPartType.NOT) {
        const notChildPostingsList = this.populatePostingsLists(queryPart.children, postingsLists)[0];
        const notPostingsList = new PostingsList();
        notPostingsList.includeInProximityRanking = false;
        let prev = 0;
        for (const td of notChildPostingsList.termDocs) {
          for (let docId = prev; docId < td.docId; docId += 1) {
            notPostingsList.termDocs.push({ docId, fields: [] });
          }
          prev = td.docId + 1;
        }
        for (let docId = prev; docId < this.numDocs; docId += 1) {
          notPostingsList.termDocs.push({ docId, fields: [] });
        }
        notPostingsList.calcPseudoIdf(this.numDocs);
        return notPostingsList;
      }

      // BRACKET
      const bracketMergedPostingsList = this.populatePostingsLists(queryPart.children, postingsLists)
        .reduce((acc, y) => (acc ? acc.merge(y) : y));
      bracketMergedPostingsList.calcPseudoIdf(this.numDocs);
      return bracketMergedPostingsList;
    });
  }

  async retrieveTopLevelPls(queryParts: QueryPart[]): Promise<PostingsList[]> {
    const postingsLists: { [term: string]: TermPostingsList } = {};
    await this.populateTermPostingsLists(queryParts, postingsLists);

    const postingsListsArray = Object.values(postingsLists);
    await Promise.all(postingsListsArray.map((pl) => pl.fetch(this.url)));

    return this.populatePostingsLists(queryParts, postingsLists);
  }
}

export default PostingsListManager;
