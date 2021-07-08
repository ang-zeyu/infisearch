import domUtils from './utils/dom';

const { h } = domUtils;

const BODY_SERP_BOUND = 40;
const MAX_SERP_HIGHLIGHT_PARTS = 2;

interface MatchResult {
  result: (string | HTMLElement)[],
  numberTermsMatched: number,
}

interface FinalMatchResult {
  result: string | HTMLElement,
  numberTermsMatched: number,
}

export function transformText(
  texts: [string, string][], // field name - field content pairs
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
): (string | HTMLElement)[] {
  const lowerCasedSortedQueryTerms = sortedQueryTerms.map((t) => t.toLowerCase());

  function getBestMatchResult(str: string): MatchResult {
    const lastTermPositions = sortedQueryTerms.map(() => -100000000);
    let lastClosestTermPositions = lastTermPositions.map((i) => i);
    let lastClosestWindowLen = 100000000;
    let lastNumberMatchedTerms = 0;

    let match = termRegex.exec(str);
    while (match) {
      const matchedText = match[2].toLowerCase();

      const matchedQueryTermIdx = lowerCasedSortedQueryTerms.findIndex(
        (term) => matchedText.includes(term),
      );
      lastTermPositions[matchedQueryTermIdx] = match.index + match[1].length;

      const validLastTermPositions = lastTermPositions.filter((p) => p >= 0);
      const windowLen = Math.max(...validLastTermPositions) - Math.min(...validLastTermPositions);

      const isMoreTermsMatched = validLastTermPositions.length > lastNumberMatchedTerms;
      if (isMoreTermsMatched || windowLen < lastClosestWindowLen) {
        if (isMoreTermsMatched) {
          lastNumberMatchedTerms = validLastTermPositions.length;
        }
        lastClosestWindowLen = windowLen;

        lastClosestTermPositions = lastTermPositions.map((pos) => pos);
      }

      match = termRegex.exec(str);
    }
    termRegex.lastIndex = 0;

    const lastClosestWindowPositions = lastClosestTermPositions
      .map((pos, idx) => ({ pos, term: sortedQueryTerms[idx] }))
      .filter((pair) => pair.pos >= 0)
      .sort((a, b) => a.pos - b.pos);
    const result: (string | HTMLElement)[] = [];
    if (!lastClosestWindowPositions.length) {
      return { result, numberTermsMatched: lastNumberMatchedTerms };
    }

    let prevHighlightEndPos = 0;
    for (let i = 0; i < lastClosestWindowPositions.length; i += 1) {
      const { pos, term } = lastClosestWindowPositions[i];
      const highlightEndPos = pos + term.length;
      if (pos > prevHighlightEndPos + BODY_SERP_BOUND * 2) {
        result.push(' ... ');
        result.push(str.substring(pos - BODY_SERP_BOUND, pos));
        result.push(h('span', { class: 'librarian-highlight' }, term));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      } else {
        result.pop();
        result.push(str.substring(prevHighlightEndPos, pos));
        result.push(h('span', { class: 'librarian-highlight' }, term));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      }
      prevHighlightEndPos = highlightEndPos;
    }
    result.push(' ...');

    return { result, numberTermsMatched: lastNumberMatchedTerms };
  }

  let lastIncludedHeading = -1;
  const finalMatchResults: FinalMatchResult[] = [];

  let itemIdx = -1;
  for (const item of texts) {
    itemIdx += 1;
    if (item[0].startsWith('heading')) {
      continue;
    }

    const { result, numberTermsMatched } = getBestMatchResult(item[1]);
    if (numberTermsMatched === 0) {
      continue;
    }

    const finalMatchResult: FinalMatchResult = { result: undefined, numberTermsMatched };
    finalMatchResults.push(finalMatchResult);

    // Find a new heading this text is under
    let i = itemIdx - 1;
    for (; i > lastIncludedHeading; i -= 1) {
      if (texts[i][0] === 'heading') {
        lastIncludedHeading = i;
        const href = (i - 1 >= 0) && texts[i - 1][0] === 'headingLink'
          ? `${baseUrl}${texts[i - 1][1]}`
          : undefined;
        finalMatchResult.result = h('a', { class: 'librarian-heading-body', href },
          h('div', { class: 'librarian-heading' }, texts[i][1]),
          h('div', { class: 'librarian-bodies' },
            h('div', { class: 'librarian-body' }, ...result)));
        break;
      }
    }

    // Insert without heading
    if (!finalMatchResult.result) {
      finalMatchResult.result = h('div', { class: 'librarian-body' }, ...result);
    }
  }

  return finalMatchResults
    .sort((a, b) => b.numberTermsMatched - a.numberTermsMatched)
    .map((r) => r.result)
    .slice(0, MAX_SERP_HIGHLIGHT_PARTS);
}

export function transformHtml(
  doc: Document,
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
): (string | HTMLElement)[] {
  const fields: [string, string][] = [];

  function traverseBody(el: HTMLElement) {
    switch (el.tagName.toLowerCase()) {
      case 'h1':
      case 'h2':
      case 'h3':
      case 'h4':
      case 'h5':
      case 'h6':
        fields.push(['heading', el.innerText]);
        break;
      default: {
        for (let i = 0; i < el.childNodes.length; i += 1) {
          const child = el.childNodes[i];
          if (child.nodeType === Node.ELEMENT_NODE) {
            traverseBody(child as HTMLElement);
          } else if (child.nodeType === Node.TEXT_NODE) {
            if (fields.length && fields[fields.length - 1][0] === 'body') {
              fields[fields.length - 1][1] += (child as Text).data;
            } else {
              fields.push(['body', (child as Text).data]);
            }
          }
        }
      }
    }
  }

  const body = doc.getElementsByTagName('body');
  if (body.length) {
    traverseBody(body[0]);
  }

  return transformText(fields, sortedQueryTerms, termRegex, baseUrl);
}
