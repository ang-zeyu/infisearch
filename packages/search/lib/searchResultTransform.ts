import * as escapeRegex from 'escape-string-regexp';
import domUtils from './utils/dom';
import Query from './results/Query';

const { h } = domUtils;

const domParser = new DOMParser();

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

/*
 Finds, cuts, and highlights the best matching excerpt
 */
function transformText(
  texts: [string, string][], // field name - field content pairs
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
): (string | HTMLElement)[] {
  const lowerCasedSortedQueryTermsIndices: { [term: string]: number } = Object.create(null);
  sortedQueryTerms.forEach((term, idx) => {
    lowerCasedSortedQueryTermsIndices[term.toLowerCase()] = idx;
  });

  function getBestMatchResult(str: string): MatchResult {
    const lastTermPositions = sortedQueryTerms.map(() => -100000000);
    let lastClosestTermPositions = lastTermPositions.map((i) => i);
    let lastClosestWindowLen = 100000000;
    let lastNumberMatchedTerms = 0;

    let match = termRegex.exec(str);
    while (match) {
      const matchedText = match[2].toLowerCase();

      const matchedQueryTermIdx = lowerCasedSortedQueryTermsIndices[matchedText];
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

/*
 Transforms a html document into field name - field content pairs
 ready for highlighting.
 */

function transformHtml(
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

/*
 Corrected / "also searched for..." terms
 */

function displayTermInfo(query: Query): HTMLElement[] {
  const misspelledTerms: string[] = [];
  const correctedTerms: string[] = [];
  const returnVal: HTMLElement[] = [];
  const correctedTermsContainer = h('div', { class: 'librarian-suggestion-container-corrected' },
    h('div', { class: 'librarian-suggestion-buttons' },
      h('button', {
        class: 'librarian-suggestion-button-dismiss',
        onclick: '() => console.log(\'hi\')',
      }),
      h('button', { class: 'librarian-suggestion-button-dismiss-tip' })));

  query.queryParts.forEach((queryPart) => {
    if (queryPart.isCorrected) {
      for (const misspelledTerm of queryPart.originalTerms) {
        if (!queryPart.terms.includes(misspelledTerm)) {
          misspelledTerms.push(misspelledTerm);
        }
      }
      for (const term of queryPart.terms) {
        correctedTerms.push(term);
      }
    } else if (queryPart.isExpanded) {
      returnVal.push(
        h('div', { class: 'librarian-suggestion-container-expanded' },
          h('div', { class: 'librarian-suggestion-content' },
            'Also searched for... ',
            h('small', {}, '(add a space to the last term to finalise the search)'),
            h('br', {}),
            ...queryPart.terms.map((expandedTerm, idx) => (idx === 0 ? '' : h(
              'span', { class: 'librarian-suggestion-expanded' }, `${expandedTerm} `,
            )))),
          h('div', { class: 'librarian-suggestion-buttons' },
            h('button', { class: 'librarian-suggestion-button-dismiss' }),
            h('button', { class: 'librarian-suggestion-button-dismiss-tip' }))),
      );
    }
  });

  if (misspelledTerms.length) {
    correctedTermsContainer.prepend(
      h('div', { class: 'librarian-suggestion-content' },
        'Could not find any matches for',
        ...misspelledTerms.map((term) => h(
          'span', { class: 'librarian-suggestion-wrong' }, ` "${term}"`,
        )),
        correctedTerms.length ? ', searched for: ' : '',
        ...correctedTerms.map((correctedTerm) => h(
          'span', { class: 'librarian-suggestion-corrected' }, `${correctedTerm} `,
        ))),
    );
    returnVal.push(correctedTermsContainer);
  }

  return returnVal;
}

/*
 Main transform function
 */

export default async function transformResults(
  query: Query,
  isFirst: boolean,
  container: HTMLElement,
  sourceHtmlFilesUrl: string,
): Promise<void> {
  const termRegex = new RegExp(
    `(^|\\W)(${query.aggregatedTerms.map((t) => `(${escapeRegex(t)})`).join('|')})(?=\\W|$)`,
    'gi',
  );

  const fragment = document.createDocumentFragment();
  const termInfoEls = isFirst ? displayTermInfo(query) : [];
  termInfoEls.forEach((el) => fragment.appendChild(el));

  let now = performance.now();

  const results = await query.retrieve(10);

  console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);
  now = performance.now();

  const resultsEls = await Promise.all(results.map(async (result) => {
    console.log(result);

    const rawLink = result.getSingleField('link');
    const fullLink = `${sourceHtmlFilesUrl}/${rawLink}`;
    let title = result.getSingleField('title') || rawLink;
    const fields = result.getStorageWithFieldNames();
    const nonTitleFields = fields.filter((v) => v[0] !== 'title');
    let bodies = transformText(
      nonTitleFields,
      query.aggregatedTerms,
      termRegex,
      rawLink,
    );

    if (!fields.find((v) => v[0] !== 'link')) {
      const asText = await (await fetch(fullLink)).text();
      const doc = domParser.parseFromString(asText, 'text/html');

      const titles = doc.getElementsByTagName('title');
      if (titles.length) {
        title = titles[0].innerText || title;
      }

      bodies = transformHtml(doc, query.aggregatedTerms, termRegex, rawLink);
    }

    return h('li', { class: 'librarian-dropdown-item' },
      h('a', { class: 'librarian-link', href: fullLink },
        h('div', { class: 'librarian-title' }, title),
        ...bodies));
  }));
  if (resultsEls.length) {
    resultsEls.forEach((el) => fragment.appendChild(el));
  } else if (isFirst) {
    fragment.appendChild(h('div', { class: 'librarian-no-results' }, 'no results found'));
  }
  const sentinel = h('li', {});
  fragment.appendChild(sentinel);

  if (isFirst) {
    container.innerHTML = '';
    container.appendChild(fragment);
  } else {
    container.appendChild(fragment);
  }

  console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  let firstRun = true;
  const iObserver = new IntersectionObserver(async (entries, observer) => {
    if (firstRun || !entries[0].isIntersecting) {
      firstRun = false;
      return;
    }

    observer.unobserve(sentinel);
    sentinel.remove();
    await transformResults(query, false, container, sourceHtmlFilesUrl);
  }, { root: container, rootMargin: '10px 10px 10px 10px' });
  iObserver.observe(sentinel);
}
