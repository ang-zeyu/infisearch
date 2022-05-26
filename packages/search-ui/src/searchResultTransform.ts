import escapeStringRegexp from 'escape-string-regexp';
import { Query } from '@morsels/search-lib';
import { FieldInfo, MorselsConfig } from '@morsels/search-lib/lib/results/Config';
import Result from '@morsels/search-lib/lib/results/Result';
import { SearchUiOptions } from './SearchUiOptions';
import createElement, { CreateElement, createInvisibleLoadingIndicator } from './utils/dom';
import { parseURL } from './utils/url';
import { InputState } from './utils/input';

const domParser = new DOMParser();

const RELATIVE_LINK_FIELD_NAME = '_relative_fp';

// How far left and right from a match to include in the body
const BODY_SERP_BOUND = 40;

interface MatchResult {
  str: string,
  /**
   * Position of the match in the string,
   * and length the match produced by the respective regex
   */
  window: { pos: number, len: number }[],
  numTerms: number,
}

/**
 * Generates the closest window (while preferring longer regex matches) in a given string.
 */
function getBestMatchResult(str: string, termRegexes: RegExp[]): MatchResult {
  // Get all matches first
  const matches = termRegexes.map(r => Array.from(str.matchAll(r)));
  if (!matches.some(innerMatches => innerMatches.length)) {
    return {
      str,
      window: [],
      numTerms: 0,
    };
  }

  // Find the closest window

  let lastClosestRegexPositions = termRegexes.map(() => -10000000);
  let lastClosestWindowLen = 10000000;
  let lastClosestRegexLengths = termRegexes.map(() => 0);

  // At each iteration, increment the lowest index match
  const matchIndices = matches.map(() => 0);
  const hasFinished =  matches.map((innerMatches) => !innerMatches.length);
  const maxMatchLengths = matches.map(() => 0);

  // Local to the while (true) loop; To avoid .map and reallocating
  const matchPositions = matches.map(() => -1);

  while (true) {
    let lowestMatchPos = 10000000;
    let lowestMatchPosExclFinished = 10000000;
    let lowestMatchIndex = -1;
    let highestMatchPos = 0;

    let hasLongerMatch = false;
    let isEqualMatch = true;
    for (let regexIdx = 0; regexIdx < matchIndices.length; regexIdx++) {
      const match = matches[regexIdx][matchIndices[regexIdx]];
      if (!match) {
        // No matches at all for this regex in this str
        continue;
      }

      // match[3] is not highlighted but allows lookahead / changing the match length priority
      const matchedTextLen = match[2].length + match[3].length;
      if (matchedTextLen > maxMatchLengths[regexIdx]) {
        // Prefer longer matches across all regexes
        hasLongerMatch = true;
        maxMatchLengths[regexIdx] = matchedTextLen;
      }
      isEqualMatch = isEqualMatch && matchedTextLen === maxMatchLengths[regexIdx];

      const pos = match.index + match[1].length;
      if (!hasFinished[regexIdx] && pos < lowestMatchPosExclFinished) {
        lowestMatchPosExclFinished = pos;
        // Find the match with the smallest position for forwarding later
        lowestMatchIndex = regexIdx;
      }
      lowestMatchPos = Math.min(lowestMatchPos, pos);
      highestMatchPos = Math.max(highestMatchPos, pos);

      matchPositions[regexIdx] = pos;
    }

    if (lowestMatchIndex === -1) {
      // hasFinished is all true
      break;
    }

    const windowLen = highestMatchPos - lowestMatchPos;
    if (hasLongerMatch || (isEqualMatch && windowLen < lastClosestWindowLen)) {
      lastClosestWindowLen = windowLen;
      lastClosestRegexPositions = [...matchPositions];
      lastClosestRegexLengths = matchIndices.map((i, idx) => matches[idx][i] && matches[idx][i][2].length);
    }

    // Forward the match with the smallest position
    matchIndices[lowestMatchIndex] += 1;
    if (matchIndices[lowestMatchIndex] >= matches[lowestMatchIndex].length) {
      hasFinished[lowestMatchIndex] = true;
      matchIndices[lowestMatchIndex] -= 1;
      if (!hasFinished.some(finished => !finished)) {
        break;
      }
    }
  }

  const window = lastClosestRegexPositions
    .map((pos, idx) => ({ pos, len: lastClosestRegexLengths[idx] }))
    .filter((pair) => pair.pos >= 0)
    .sort((a, b) => a.pos - b.pos);
  const numTerms = window.length;
  return { str, window, numTerms };
}

function createEllipses() {
  return createElement('span', { class: 'morsels-ellipsis', 'aria-label': 'ellipses' }, ' ... ');
}

/**
 * Generates the HTML preview of the match result given.
 */
function highlightMatchResult(
  matchResult: MatchResult,
  addEllipses: boolean,
  options: SearchUiOptions,
): (string | HTMLElement)[] {
  const { highlightRender } = options.uiOptions.resultsRenderOpts;
  const { str, window } = matchResult;

  if (!window.some(({ pos }) => pos >= 0)) {
    if (addEllipses) {
      return [str.trimStart().substring(0, BODY_SERP_BOUND * 2), createEllipses()];
    } else {
      return [str];
    }
  }

  const result: (string | HTMLElement)[] = [];
  let prevHighlightEndPos = 0;
  for (const { pos, len } of window) {
    const highlightEndPos = pos + len;
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

interface ProcessedMatchResult extends MatchResult {
  pairIdx: number,
  headingMatch?: MatchResult,
  headingLink?: string,
}

/**
 * Finds, cuts, and highlights the best matching excerpt of 'heading' and 'body' fields
 * @param texts array of ['field name', 'field content'] pairs
 */
function transformText(
  texts: [string, string][],
  termRegexes: RegExp[],
  baseUrl: string,
  options: SearchUiOptions,
): (string | HTMLElement)[] {
  const {
    bodyOnlyRender,
    headingBodyRender,
  } = options.uiOptions.resultsRenderOpts;

  let lastHeadingMatch: ProcessedMatchResult = undefined;
  let lastHeadingLinkIdx = -2;
  let lastHeadingLinkText = '';
  let matchResults: ProcessedMatchResult[] = [];

  for (let pairIdx = 0; pairIdx < texts.length; pairIdx += 1) {
    const [fieldName, fieldText] = texts[pairIdx];
    switch (fieldName) {
      case 'headingLink': {
        lastHeadingLinkIdx = pairIdx;
        lastHeadingLinkText = fieldText;
        break;
      }
      case 'heading': {
        lastHeadingMatch = getBestMatchResult(fieldText, termRegexes) as ProcessedMatchResult;
        lastHeadingMatch.pairIdx = pairIdx;
        lastHeadingMatch.headingLink = lastHeadingLinkIdx === lastHeadingMatch.pairIdx - 1
          ? lastHeadingLinkText
          : '';
        
        // Push a heading-only match in case there are no other matches (unlikely).
        matchResults.push({
          str: '',
          window: [],
          numTerms: -2000, // even less preferable than body-only matches
          headingMatch: lastHeadingMatch,
          headingLink: lastHeadingMatch.headingLink,
          pairIdx: pairIdx,
        });
        break;
      }
      case 'body': {
        const finalMatchResult = getBestMatchResult(fieldText, termRegexes) as ProcessedMatchResult;
        if (lastHeadingMatch) {
          // Link up body matches with headings, headingLinks if any
          finalMatchResult.headingMatch = lastHeadingMatch;
          finalMatchResult.headingLink = lastHeadingMatch.headingLink;
          finalMatchResult.numTerms += lastHeadingMatch.numTerms;
        } else {
          // body-only match, add an offset to prefer heading-body matches
          finalMatchResult.numTerms -= 1000;
        }
        matchResults.push(finalMatchResult);
        break;
      }
    }
  }

  matchResults.sort((a, b) => {
    return a.numTerms === 0 && b.numTerms === 0
      // If there are 0 terms matched for both matches, prefer "longer" snippets
      ? b.str.length - a.str.length
      : b.numTerms - a.numTerms;
  });

  const matches = [];
  const maxMatches = Math.min(matchResults.length, 2); // maximum 2 for now
  for (let i = 0; i < maxMatches; i += 1) {
    if (matchResults[i].numTerms !== matchResults[0].numTerms) {
      break;
    }
    matches.push(matchResults[i]);
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

/**
 * Transforms a html document into field name - field content pairs
 * ready for use in transformText.
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

  // DFS
  function traverse(el: HTMLElement, fieldName: string) {
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
          traverse(child as HTMLElement, fieldName);
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

  traverse(doc.documentElement, undefined);

  const titleField = fields.find((pair) => pair[0] === 'title');
  let title = '';
  if (titleField) {
    [,title] = titleField;
  }

  return {
    title,
    bodies: transformText(
      fields, termRegexes, baseUrl, options,
    ),
  };
}


/**
 * Determines from where (source files / field stores) to retrieve the document's fields.
 * Then calls one of the transformXx variants above.
 */
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
      fields,
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
        return escapeStringRegexp(t);
      })
      .sort((a, b) => b.length - a.length)
      .join('|');

    // A little hardcoded, not so pretty but gets the job done for now
    if (config.langConfig.lang === 'ascii') {
      const boundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})((?=\\W|$))`, 'gi');
      termRegexes.push(boundariedRegex);
    } else if (config.langConfig.lang === 'latin') {
      const nonEndBoundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})(\\W?)`, 'gi');
      termRegexes.push(nonEndBoundariedRegex);
    } else if (config.langConfig.lang === 'chinese') {
      const nonBoundariedRegex = new RegExp(`()(${innerTermsJoined})()`, 'gi');
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

/**
 * @returns Whether the results were computed and displayed, or pre-emptively disrupted by a new query
 */
export default async function transformResults(
  inputState: InputState,
  query: Query,
  config: MorselsConfig,
  isFirst: boolean,
  container: HTMLElement,
  topLoader: { v: HTMLElement },
  options: SearchUiOptions,
): Promise<boolean> {
  if (inputState.nextAction) {
    // If a new query interrupts the current one
    return false;
  }

  const bottomLoader = options.uiOptions.loadingIndicatorRender(createElement, options, false, false);
  if (!isFirst) {
    container.appendChild(bottomLoader);
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

  if (inputState.nextAction) {
    // If a new query interrupts the current one
    return false;
  }

  const resultsEls = await options.uiOptions.resultsRender(
    createElement, options, config, results, query,
  );

  if (inputState.nextAction) {
    // If a new query interrupts the current one
    return false;
  }

  if (resultsEls.length) {
    resultsEls.forEach((el) => fragment.appendChild(el));
  } else if (isFirst) {
    fragment.appendChild(options.uiOptions.noResultsRender(createElement, options));
  }
  const sentinel = fragment.lastElementChild;

  if (isFirst) {
    container.innerHTML = '';
    topLoader.v = createInvisibleLoadingIndicator();
    container.append(topLoader.v);
    container.append(fragment);
  } else {
    bottomLoader.replaceWith(fragment);
  }

  //console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  if (resultsEls.length) {
    inputState.lastElObserver = new IntersectionObserver(async (entries, observer) => {
      if (!entries[0].isIntersecting) {
        return;
      }
  
      observer.unobserve(sentinel);
      await transformResults(inputState, query, config, false, container, topLoader, options);
    }, { rootMargin: '10px 10px 10px 10px' });

    inputState.lastElObserver.observe(sentinel);
  }

  return true;
}
