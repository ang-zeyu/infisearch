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
  str: string,
  lastClosestWindowPositions: { pos: number, idx: number }[],
  lastClosestTermLengths: number[],
  numberTermsMatched: number,
}

function getBestMatchResult(str: string, termRegexes: RegExp[]): MatchResult {
  // Get all matches first
  const matches = termRegexes.map(r => Array.from(str.matchAll(r)));
  if (!matches.some(innerMatches => innerMatches.length)) {
    return {
      str,
      lastClosestTermLengths: [],
      lastClosestWindowPositions: [],
      numberTermsMatched: 0,
    };
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
  const numberTermsMatched = lastClosestWindowPositions.length;
  return { str, lastClosestWindowPositions, lastClosestTermLengths, numberTermsMatched };
}

interface FinalMatchResult extends MatchResult {
  itemIdx: number,
  headingMatch?: MatchResult,
  headingLink?: string,
}

function createEllipses() {
  return createElement('span', { class: 'morsels-ellipsis', 'aria-label': 'ellipses' }, ' ... ');
}

function highlightMatchResult(
  matchResult: MatchResult,
  addEllipses: boolean,
  options: SearchUiOptions,
): (string | HTMLElement)[] {
  const { highlightRender } = options.uiOptions.resultsRenderOpts;
  const { str, lastClosestWindowPositions, lastClosestTermLengths } = matchResult;

  if (!lastClosestWindowPositions.some(({ pos }) => pos >= 0)) {
    if (addEllipses) {
      return [str.trimStart().substring(0, BODY_SERP_BOUND * 2), createEllipses()];
    } else {
      return [str];
    }
  }

  const result: (string | HTMLElement)[] = [];
  let prevHighlightEndPos = 0;
  for (const { pos, idx } of lastClosestWindowPositions) {
    const highlightEndPos = pos + lastClosestTermLengths[idx];
    if (pos > prevHighlightEndPos + BODY_SERP_BOUND * 2) {
      if (addEllipses) {
        result.push(createEllipses());
      }
      result.push(str.substring(pos - BODY_SERP_BOUND, pos));
      result.push(highlightRender(createElement, options, str.substring(pos, highlightEndPos)));
    } else if (pos >= prevHighlightEndPos) {
      result.pop();
      result.push(str.substring(prevHighlightEndPos, pos));
      result.push(highlightRender(createElement, options, str.substring(pos, highlightEndPos)));
    } else {
      // Intersecting matches
      if (highlightEndPos > prevHighlightEndPos) {
        result.pop();
        const previousHighlight = result[result.length - 1] as HTMLElement;
        previousHighlight.textContent += str.substring(prevHighlightEndPos, highlightEndPos);
      } else {
        // The highlight is already contained within the previous highlight
        continue;
      }
    }
    result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));

    prevHighlightEndPos = highlightEndPos;
  }

  if (addEllipses) {
    result.push(createEllipses());
  }

  return result;
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
    bodyOnlyRender,
    headingBodyRender,
  } = options.uiOptions.resultsRenderOpts;

  let lastHeadingMatch: FinalMatchResult = undefined;
  let lastHeadingLink = {
    itemIdx: -100,
    fieldText: '',
  };
  let finalMatchResults: FinalMatchResult[] = [];

  for (let itemIdx = 0; itemIdx < texts.length; itemIdx += 1) {
    const [fieldName, fieldText] = texts[itemIdx];
    if (fieldName === 'headingLink') {
      lastHeadingLink = {
        itemIdx,
        fieldText,
      };
      continue;
    }

    const matchResult = getBestMatchResult(fieldText, termRegexes);
    if (fieldName === 'heading') {
      lastHeadingMatch = matchResult as FinalMatchResult;
      lastHeadingMatch.itemIdx = itemIdx;
      lastHeadingMatch.headingLink = lastHeadingLink.itemIdx === lastHeadingMatch.itemIdx - 1
        ? lastHeadingLink.fieldText
        : '';
      
      // Push a heading-only match in case there are no other matches (unlikely).
      finalMatchResults.push({
        str: '',
        lastClosestTermLengths: [],
        lastClosestWindowPositions: [],
        numberTermsMatched: -2000, // even less preferable than body-only matches
        headingMatch: lastHeadingMatch,
        headingLink: lastHeadingMatch.headingLink,
        itemIdx,
      });
    } else if (fieldName === 'body') {
      const finalMatchResult = matchResult as FinalMatchResult;
      if (lastHeadingMatch) {
        // Link up body matches with headings, headingLinks if any
        finalMatchResult.headingMatch = lastHeadingMatch;
        finalMatchResult.headingLink = lastHeadingMatch.headingLink;
        finalMatchResult.numberTermsMatched += lastHeadingMatch.numberTermsMatched;
      } else {
        // body-only match, add an offset to prefer heading-body matches
        finalMatchResult.numberTermsMatched -= 1000;
      }
      finalMatchResults.push(finalMatchResult);
    }
  }

  finalMatchResults.sort((a, b) => {
    return a.numberTermsMatched === b.numberTermsMatched && a.numberTermsMatched === 0
      // If there are 0 terms matched for both matches, prefer "longer snippets"
      ? b.str.length - a.str.length
      : b.numberTermsMatched - a.numberTermsMatched;
  });

  const matches = [];
  const maxMatches = Math.min(finalMatchResults.length, MAX_SERP_HIGHLIGHT_PARTS);
  for (let i = 0; i < maxMatches; i += 1) {
    if (finalMatchResults[i].numberTermsMatched != finalMatchResults[0].numberTermsMatched) {
      break;
    }
    matches.push(finalMatchResults[i]);
  }

  return matches.map((finalMatchResult) => {
    const bodyHighlights = highlightMatchResult(finalMatchResult, true, options);
    if (finalMatchResult.headingMatch) {
      const highlightedHeadings = highlightMatchResult(finalMatchResult.headingMatch, false, options);
      const headingHighlights = highlightedHeadings.length
        ? highlightedHeadings
        : [finalMatchResult.headingMatch.str];
      const href = finalMatchResult.headingLink && `${baseUrl}#${finalMatchResult.headingLink}`;
      return headingBodyRender(
        createElement,
        options,
        headingHighlights,
        bodyHighlights,
        href,
      );
    } else {
      return bodyOnlyRender(createElement, options, bodyHighlights);
    }
  });
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

  const loader = options.uiOptions.loadingIndicatorRender(createElement, options, false, Promise.resolve());
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
