export enum QueryPartType {
  TERM = 'TERM',
  PHRASE = 'PHRASE',
  BRACKET = 'BRACKET',
  AND = 'AND',
  NOT = 'NOT',
  ADDED = 'ADDED',
}

export interface QueryPart {
  isCorrected?: boolean;
  shouldExpand?: boolean;
  isExpanded?: boolean;
  originalTerms?: string[];
  type: QueryPartType;
  terms?: string[];
  children?: QueryPart[];
  weight?: number;
}

enum QueryParseState {
  NONE,
  QUOTE,
  PARENTHESES,
}

const whitespace = new RegExp('\\s');

export default function parseQuery(query: string, tokenize: (string) => string[]): QueryPart[] {
  const queryParts: QueryPart[] = [];

  let queryParseState: QueryParseState = QueryParseState.NONE;
  let isExpectingAnd = false;
  let isUnaryOperatorAllowed = true;
  let didEncounterNot = false;

  let i = 0;
  let j = 0;

  function wrapInNot(queryPart: QueryPart): QueryPart {
    if (didEncounterNot) {
      didEncounterNot = false;
      return {
        type: QueryPartType.NOT,
        children: [queryPart],
      };
    }
    return queryPart;
  }

  function handleFreeText(
    text: string,
  ) {
    if (i === j) {
      return;
    }

    const terms = tokenize(text.slice(i, j));
    if (!terms.length) {
      return;
    }

    queryParts.push(wrapInNot({
      type: QueryPartType.TERM,
      terms: [terms.shift()],
      weight: 1,
    }));

    for (const term of terms) {
      queryParts.push({
        type: QueryPartType.TERM,
        terms: [term],
        weight: 1,
      });
    }
  }

  for (; j < query.length; j += 1) {
    const c = query[j];

    switch (queryParseState) {
      case QueryParseState.QUOTE: {
        if (c === '"') {
          queryParseState = QueryParseState.NONE;

          const terms = tokenize(query.slice(i, j));
          const phraseQueryPart: QueryPart = wrapInNot({
            type: terms.length <= 1 ? QueryPartType.TERM : QueryPartType.PHRASE,
            terms,
          });

          if (isExpectingAnd) {
            queryParts[queryParts.length - 1].children.push(phraseQueryPart);
            isExpectingAnd = false;
          } else {
            queryParts.push(phraseQueryPart);
          }

          i = j + 1;

          isUnaryOperatorAllowed = true;
        }
        break;
      }
      case QueryParseState.PARENTHESES: {
        if (c === ')') {
          queryParseState = QueryParseState.NONE;

          const childQueryPart: QueryPart = wrapInNot({
            type: QueryPartType.BRACKET,
            children: parseQuery(query.slice(i, j), tokenize),
          });

          if (isExpectingAnd) {
            queryParts[queryParts.length - 1].children.push(childQueryPart);
            isExpectingAnd = false;
          } else {
            queryParts.push(childQueryPart);
          }

          i = j + 1;

          isUnaryOperatorAllowed = true;
        }
        break;
      }
      case QueryParseState.NONE: {
        if (c === '"' || c === '(') {
          if (isExpectingAnd) {
            if (i !== j) {
              const currQueryParts = parseQuery(query.slice(i, j), tokenize);
              queryParts[queryParts.length - 1].children.push(wrapInNot(currQueryParts.shift()));
              queryParts.push(...currQueryParts);
              isExpectingAnd = false;
            }
            // i === j: the phrase / parentheses is part of the AND (e.g. lorem AND (ipsum))
          } else {
            handleFreeText(query);
          }
          queryParseState = c === '"' ? QueryParseState.QUOTE : QueryParseState.PARENTHESES;
          i = j + 1;
        } else if (whitespace.test(c)) {
          const initialJ = j;
          while (query[j] && whitespace.test(query[j])) {
            j += 1;
          }

          if (
            j < query.length - 4
            && query[j] === 'A' && query[j + 1] === 'N' && query[j + 2] === 'D'
            && whitespace.test(query[j + 3])
          ) {
            const currQueryParts = parseQuery(query.slice(i, initialJ), tokenize);
            if (currQueryParts.length) {
              currQueryParts[0] = wrapInNot(currQueryParts[0]);

              if (isExpectingAnd) {
                queryParts[queryParts.length - 1].children = [
                  ...queryParts[queryParts.length - 1].children,
                  currQueryParts.shift(),
                ];
              }

              if (currQueryParts.length) {
                // A new, disjoint AND group from the previous (if any)
                const lastCurrQueryPart = currQueryParts.pop();
                queryParts.push(...currQueryParts);

                queryParts.push({
                  type: QueryPartType.AND,
                  children: [lastCurrQueryPart],
                });
              }
            } else if (queryParts.length && !isExpectingAnd) {
              // e.g. (lorem) AND ipsum
              queryParts.push({
                type: QueryPartType.AND,
                children: [queryParts.pop()],
              });
            } else {
              throw new Error('Query parsing error: no token found before AND operator');
            }
            isExpectingAnd = true;

            j += 4;
            while (query[j] && whitespace.test(query[j])) {
              j += 1;
            }
            i = j;
          }

          j -= 1;

          isUnaryOperatorAllowed = true;
        } else if (
          isUnaryOperatorAllowed
          && j < query.length - 4
          && query[j] === 'N' && query[j + 1] === 'O' && query[j + 2] === 'T'
          && whitespace.test(query[j + 3])
        ) {
          const currQueryParts = parseQuery(query.slice(i, j), tokenize);
          if (currQueryParts.length) {
            currQueryParts[0] = wrapInNot(currQueryParts[0]);

            if (isExpectingAnd) {
              queryParts[queryParts.length - 1].children = [
                ...queryParts[queryParts.length - 1].children,
                currQueryParts.shift(),
              ];
              isExpectingAnd = false;
            }

            queryParts.push(...currQueryParts);
          }
          didEncounterNot = true;

          j += 4;
          while (whitespace.test(query[j])) {
            j += 1;
          }
          i = j;
          j -= 1;
        } else {
          isUnaryOperatorAllowed = false;
        }
        break;
      }
      default:
        return undefined;
    }
  }

  if (isExpectingAnd) {
    if (i !== j) {
      const lastQueryParts = parseQuery(query.slice(i, j), tokenize);
      queryParts[queryParts.length - 1].children.push(wrapInNot(lastQueryParts.shift()));
      queryParts.push(...lastQueryParts);
    } else {
      throw new Error('Query parsing error: no token found after AND operator');
    }
  } else {
    handleFreeText(query);
  }

  return queryParts;
}
