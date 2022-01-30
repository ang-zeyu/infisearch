import escapeStringRegexp from 'escape-string-regexp';
import { Query } from '@morsels/search-lib';
import { FieldInfo, MorselsConfig } from '@morsels/search-lib/lib/results/FieldInfo';
import Result from '@morsels/search-lib/lib/results/Result';
import { SearchUiOptions } from './SearchUiOptions';
import createElement, { CreateElement } from './utils/dom';
import { parseURL } from './utils/url';
import { InputState } from './utils/input';

const domParser = new DOMParser();

const RELATIVE_LINK_FIELD_NAME = '_relative_fp';

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

function createEllipses() {
  return createElement('span', { class: 'morsels-ellipsis', 'aria-label': 'ellipses' }, ' ... ');
}

/*
 Finds, cuts, and highlights the best matching excerpt
 */
function transformText(
  texts: [string, string][], // field name - field content pairs
  termRegexes: RegExp[],
  baseUrl: string,
  options: SearchUiOptions,
): (string | HTMLElement)[] {
  const {
    highlightRender,
    bodyOnlyRender,
    headingBodyRender,
  } = options.uiOptions.resultsRenderOpts;

  let bestNumTermMatches: number = 0;

  function getBestMatchResult(str: string): MatchResult {
    // Get all matches first
    const matches = termRegexes.map(r => Array.from(str.matchAll(r)));
    if (!matches.some(innerMatches => innerMatches.length)) {
      return { result: [], numberTermsMatched: 0 };
    }

    // Find the closest window

    let lastClosestTermPositions = termRegexes.map(() => -100000000);
    let lastClosestWindowLen = 100000000;
    let lastClosestTermLengths = termRegexes.map(() => 0);

    // At each iteration, increment the lowest index match
    const matchIndices = matches.map(() => 0);
    const hasFinished =  matches.map((innerMatches) => !innerMatches.length);
    const maxMatchLengths = matches.map(() => 0);

    // Local to the loop; To avoid .map and reallocating
    const matchPositions = matches.map(() => -1);

    while (true) {
      let lowestMatchPos = 10000000000;
      let lowestMatchPosExclFinished = 10000000000;
      let lowestMatchIndex = undefined;
      let highestMatchPos = 0;

      let hasLongerMatch = false;
      let isEqualMatch = true;
      for (let idx = 0; idx < matchIndices.length; idx++) {
        const match = matches[idx][matchIndices[idx]];
        if (!match) {
          // No matches at all for this regex in this str
          continue;
        }

        const matchedText = match[2];
        if (matchedText.length > maxMatchLengths[idx]) {
          // Prefer longer matches across all regexes
          hasLongerMatch = true;
          maxMatchLengths[idx] = matchedText.length;
        }
        isEqualMatch = isEqualMatch && matchedText.length === maxMatchLengths[idx];

        const pos = match.index;
        if (!hasFinished[idx] && pos < lowestMatchPosExclFinished) {
          lowestMatchPosExclFinished = pos;
          // Find the match with the smallest position for forwarding later
          lowestMatchIndex = idx;
        }
        lowestMatchPos = Math.min(lowestMatchPos, pos);
        highestMatchPos = Math.max(highestMatchPos, pos);

        matchPositions[idx] = pos;
      }

      const windowLen = highestMatchPos - lowestMatchPos;
      if (
        hasLongerMatch
        // If all matches are equally long as before, prefer the smaller window
        || (isEqualMatch && windowLen < lastClosestWindowLen)
      ) {
        lastClosestWindowLen = windowLen;
        lastClosestTermPositions = [...matchPositions];
        lastClosestTermLengths = matchIndices.map((i, idx) => matches[idx][i] && matches[idx][i][0].length);
      }

      // Forward the match with the smallest position
      if (lowestMatchIndex !== undefined) {
        matchIndices[lowestMatchIndex] += 1;
        if (matchIndices[lowestMatchIndex] >= matches[lowestMatchIndex].length) {
          hasFinished[lowestMatchIndex] = true;
          matchIndices[lowestMatchIndex] -= 1;
          if (!hasFinished.some(finished => !finished)) {
            break;
          }
        }
      } else {
        break;
      }
    }


    const lastClosestWindowPositions = lastClosestTermPositions
      .map((pos, idx) => ({ pos, idx }))
      .filter((pair) => pair.pos >= 0)
      .sort((a, b) => a.pos - b.pos);
    const result: (string | HTMLElement)[] = [];
    const numberTermsMatched = lastClosestWindowPositions.length;
    if (numberTermsMatched < bestNumTermMatches) {
      return { result, numberTermsMatched };
    }

    let prevHighlightEndPos = 0;
    for (let i = 0; i < lastClosestWindowPositions.length; i += 1) {
      const { pos, idx } = lastClosestWindowPositions[i];
      const highlightEndPos = pos + lastClosestTermLengths[idx];
      if (pos > prevHighlightEndPos + BODY_SERP_BOUND * 2) {
        result.push(createEllipses());
        result.push(str.substring(pos - BODY_SERP_BOUND, pos));
        result.push(highlightRender(createElement, options, str.substring(pos, highlightEndPos)));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      } else if (pos >= prevHighlightEndPos) {
        result.pop();
        result.push(str.substring(prevHighlightEndPos, pos));
        result.push(highlightRender(createElement, options, str.substring(pos, highlightEndPos)));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      } else {
        continue;
      }
      prevHighlightEndPos = highlightEndPos;
    }
    result.push(createEllipses());

    return { result, numberTermsMatched };
  }

  let lastIncludedHeading = -1;
  let bestBodyMatch: FinalMatchResult = { result: undefined, numberTermsMatched: 0 };
  let finalMatchResults: FinalMatchResult[] = [];

  let itemIdx = -1;
  for (const item of texts) {
    itemIdx += 1;
    if (item[0].startsWith('heading')) {
      continue;
    }

    const { result, numberTermsMatched } = getBestMatchResult(item[1]);
    if (numberTermsMatched === 0 || numberTermsMatched < bestNumTermMatches) {
      continue;
    } else if (numberTermsMatched > bestNumTermMatches) {
      finalMatchResults = [];
      bestNumTermMatches = numberTermsMatched;
    }

    // Find a new heading this text is under
    let i = itemIdx - 1;
    for (; i > lastIncludedHeading; i -= 1) {
      if (texts[i][0] === 'heading') {
        lastIncludedHeading = i;

        finalMatchResults.push({
          result: headingBodyRender(
            createElement,
            options,
            texts[i][1],
            result,
            (i - 1 >= 0)
              && texts[i - 1][0] === 'headingLink'
              && `${baseUrl}#${texts[i - 1][1]}`,
          ),
          numberTermsMatched,
        });
        break;
      }
    }

    // Insert without heading; Prefer matches under headings
    if (!finalMatchResults.length && numberTermsMatched > bestBodyMatch.numberTermsMatched) {
      bestBodyMatch = {
        result: bodyOnlyRender(createElement, options, result),
        numberTermsMatched,
      };
    }
  }

  if (!finalMatchResults.length && bestBodyMatch.numberTermsMatched > 0) {
    finalMatchResults.push(bestBodyMatch);
  }

  return finalMatchResults
    .map((r) => r.result)
    .slice(0, MAX_SERP_HIGHLIGHT_PARTS);
}

function transformJson(
  json: any,
  loaderConfig: any,
  termRegexes: RegExp[],
  baseUrl: string,
  options: SearchUiOptions,
) {
  const fields: [string, string][] = [];

  // eslint-disable-next-line @typescript-eslint/naming-convention
  const { field_map, field_order } = loaderConfig;

  const titleEntry = Object.entries(field_map).find(([, indexedFieldName]) => indexedFieldName === 'title');
  const titleKey = titleEntry && titleEntry[0];

  for (const field of field_order) {
    if (field !== titleKey && json[field]) {
      fields.push([
        field_map[field],
        json[field],
      ]);
    }
  }

  return {
    title: titleKey && json[titleKey],
    bodies: transformText(fields, termRegexes, baseUrl, options),
  };
}

/*
 Transforms a html document into field name - field content pairs
 ready for highlighting.
 */

function transformHtml(
  doc: Document,
  loaderConfig: any,
  termRegexes: RegExp[],
  baseUrl: string,
  options: SearchUiOptions,
): { title: string, bodies: (string | HTMLElement)[] } {
  const fields: [string, string][] = [];

  if (loaderConfig.exclude_selectors) {
    for (const excludeSelector of loaderConfig.exclude_selectors) {
      const nodes = doc.querySelectorAll(excludeSelector);
      for (let i = 0; i < nodes.length; i += 1) {
        nodes[i].remove();
      }
    }
  }

  loaderConfig.selectors = loaderConfig.selectors || [];
  const allSelectors = loaderConfig.selectors.map((s) => s.selector).join(',');

  function traverseBody(el: HTMLElement, fieldName: string) {
    for (const selector of loaderConfig.selectors) {
      if (el.matches(selector.selector)) {
        Object.entries(selector.attr_map).forEach(([attrName, attrFieldName]) => {
          if (el.attributes[attrName]) {
            fields.push([attrFieldName as any, el.attributes[attrName].value]);
          }
        });

        // eslint-disable-next-line no-param-reassign
        fieldName = selector.field_name;
        break;
      }
    }

    if (el.querySelector(allSelectors)) {
      for (let i = 0; i < el.childNodes.length; i += 1) {
        const child = el.childNodes[i];
        if (child.nodeType === Node.ELEMENT_NODE) {
          traverseBody(child as HTMLElement, fieldName);
        } else if (child.nodeType === Node.TEXT_NODE && fieldName) {
          if (fields.length && fields[fields.length - 1][0] === fieldName) {
            fields[fields.length - 1][1] += (child as Text).data;
          } else {
            fields.push([fieldName, (child as Text).data]);
          }
        }
      }
    } else if (fieldName) {
      // Fast track
      if (fields.length && fields[fields.length - 1][0] === fieldName) {
        fields[fields.length - 1][1] += el.textContent;
      } else {
        fields.push([fieldName, el.textContent || '']);
      }
    }
  }

  traverseBody(doc.documentElement, undefined);

  const titleField = fields.find((pair) => pair[0] === 'title');
  let title = '';
  if (titleField) {
    [,title] = titleField;
    titleField[1] = '';
  }

  return {
    title,
    bodies: transformText(
      fields, termRegexes, baseUrl, options,
    ),
  };
}

/*
 Main transform function
 */

const nonContentFields = new Set([RELATIVE_LINK_FIELD_NAME, 'title', 'link']);

async function singleResultRender(
  result: Result,
  options: SearchUiOptions,
  configs: MorselsConfig,
  hasStoredContentField: FieldInfo,
  query: Query,
  searchedTermsJSON: string,
  termRegexes: RegExp[],
) {
  const { loaderConfigs } = configs.indexingConfig;

  const fields = result.getStorageWithFieldNames();

  let link: string;
  let relativeLink: string;
  let resultTitle: string;
  for (const fieldNameAndField of fields) {
    const [fieldName, fieldText] = fieldNameAndField;
    switch (fieldName) {
      case 'link':
        link = fieldText;
        break;
      case RELATIVE_LINK_FIELD_NAME:
        relativeLink = fieldText;
        break;
      case 'title':
        resultTitle = fieldText;
        break;
    }
    if (link && relativeLink && resultTitle) {
      break;
    }
  }
  const hasSourceFilesUrl = typeof options.uiOptions.sourceFilesUrl === 'string';
  const fullLink = link
    || (hasSourceFilesUrl && relativeLink && `${options.uiOptions.sourceFilesUrl}${relativeLink}`)
    || '';

  resultTitle = resultTitle || relativeLink || link;

  let linkToAttach = fullLink;
  if (options.uiOptions.resultsRenderOpts.addSearchedTerms && fullLink) {
    const fullLinkUrl = parseURL(fullLink);
    fullLinkUrl.searchParams.append(
      options.uiOptions.resultsRenderOpts.addSearchedTerms,
      searchedTermsJSON,
    );
    linkToAttach = fullLinkUrl.toString();
  }

  let resultHeadingsAndTexts: (string | HTMLElement)[];
  if (hasStoredContentField) {
    resultHeadingsAndTexts = transformText(
      fields.filter((v) => !nonContentFields.has(v[0])),
      termRegexes,
      linkToAttach,
      options,
    );
  } else if (!fullLink) {
    // Unable to retrieve and load from source file
    resultHeadingsAndTexts = [];
  } else if (fullLink.endsWith('.html') && loaderConfigs.HtmlLoader) {
    const asText = await (await fetch(fullLink)).text();
    const doc = domParser.parseFromString(asText, 'text/html');

    const { title: newTitle, bodies: newHeadingsAndTexts } = transformHtml(
      doc, loaderConfigs.HtmlLoader, termRegexes, linkToAttach, options,
    );
    resultTitle = newTitle || resultTitle;
    resultHeadingsAndTexts = newHeadingsAndTexts;
  } else if (fullLink.endsWith('.txt') && loaderConfigs.TxtLoader) {
    const asText = await (await fetch(fullLink)).text();
    resultHeadingsAndTexts = transformText(
      [['body', asText]], termRegexes, linkToAttach, options,
    );
  } else {
    const fullLinkUrl = parseURL(fullLink);
    if (fullLinkUrl.pathname.endsWith('.json') && loaderConfigs.JsonLoader) {
      const asJson = await (await fetch(fullLink)).json();

      const { title: newTitle, bodies: newBodies } = transformJson(
        fullLinkUrl.hash ? asJson[fullLinkUrl.hash.substring(1)] : asJson,
        loaderConfigs.JsonLoader,
        termRegexes, linkToAttach, options,
      );
      resultTitle = newTitle || resultTitle;
      resultHeadingsAndTexts = newBodies;
    }
  }

  return options.uiOptions.resultsRenderOpts.listItemRender(
    createElement,
    options,
    searchedTermsJSON,
    fullLink,
    resultTitle,
    resultHeadingsAndTexts,
    fields,
  );
}

export function resultsRender(
  h: CreateElement,
  options: SearchUiOptions,
  config: MorselsConfig,
  results: Result[],
  query: Query,
): Promise<HTMLElement[]> {
  const termRegexes: RegExp[] = [];
  const searchedTerms: string[] = [];
  for (const innerTerms of query.searchedTerms) {
    const innerTermsJoined = innerTerms
      .map(t => {
        searchedTerms.push(t);
        return `(${escapeStringRegexp(t)})`;
      })
      .sort((a, b) => b.length - a.length)
      .join('|');

    // A little hardcoded, not so pretty but gets the job done for now
    if (config.langConfig.lang === 'ascii') {
      const boundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})(?=\\W|$)`, 'gi');
      termRegexes.push(boundariedRegex);
    } else if (config.langConfig.lang === 'latin') {
      const nonEndBoundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined}\\W?)`, 'gi');
      termRegexes.push(nonEndBoundariedRegex);
    } else if (config.langConfig.lang === 'chinese') {
      const nonBoundariedRegex = new RegExp(`()(${innerTermsJoined})`, 'gi');
      termRegexes.push(nonBoundariedRegex);
    }
  }

  const hasStoredContentField = config.fieldInfos.find((info) => info.do_store
      && (info.name === 'body' || info.name === 'title' || info.name === 'heading'));

  return Promise.all(results.map(
    (result) => singleResultRender(
      result, options, config, hasStoredContentField, query, JSON.stringify(searchedTerms), termRegexes,
    ),
  ));
}

export default async function transformResults(
  inputState: InputState,
  query: Query,
  config: MorselsConfig,
  isFirst: boolean,
  container: HTMLElement,
  options: SearchUiOptions,
): Promise<void> {
  if (query !== inputState.currQuery) {
    // If a new query interrupts the current one
    return;
  }

  const loader = options.uiOptions.loadingIndicatorRender(createElement, options);
  if (!isFirst) {
    container.appendChild(loader);
  }

  if (inputState.lastElObserver) {
    inputState.lastElObserver.disconnect();
  }

  const fragment = document.createDocumentFragment();
  const termInfoEls = isFirst
    ? options.uiOptions.termInfoRender(createElement, options, query.queryParts)
    : [];
  termInfoEls.forEach((el) => fragment.appendChild(el));

  //let now = performance.now();

  const results = await query.getNextN(options.uiOptions.resultsPerPage);

  //console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);
  //now = performance.now();

  if (query !== inputState.currQuery) {
    // If a new query interrupts the current one
    return;
  }

  const resultsEls = await options.uiOptions.resultsRender(
    createElement, options, config, results, query,
  );

  if (query !== inputState.currQuery) {
    // If a new query interrupts the current one
    return;
  }

  if (resultsEls.length) {
    resultsEls.forEach((el) => fragment.appendChild(el));
  } else if (isFirst) {
    fragment.appendChild(options.uiOptions.noResultsRender(createElement, options));
  }
  const sentinel = fragment.lastElementChild;

  if (isFirst) {
    container.innerHTML = '';
    container.appendChild(fragment);
  } else {
    loader.replaceWith(fragment);
  }

  //console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  if (resultsEls.length) {
    inputState.lastElObserver = new IntersectionObserver(async (entries, observer) => {
      if (!entries[0].isIntersecting) {
        return;
      }
  
      observer.unobserve(sentinel);
      await transformResults(inputState, query, config, false, container, options);
    }, { rootMargin: '10px 10px 10px 10px' });

    inputState.lastElObserver.observe(sentinel);
  }
}
