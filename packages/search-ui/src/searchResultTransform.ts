import * as escapeRegex from 'escape-string-regexp';
import { Query } from '@morsels/search-lib';
import { FieldInfo, MorselsConfig } from '@morsels/search-lib/lib/results/FieldInfo';
import { QueryPart } from '@morsels/search-lib/lib/parser/queryParser';
import Result from '@morsels/search-lib/lib/results/Result';
import { ArbitraryRenderOptions, SearchUiOptions, SearchUiRenderOptions } from './SearchUiOptions';
import createElement, { CreateElement } from './utils/dom';

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

/*
 Finds, cuts, and highlights the best matching excerpt
 */
function transformText(
  texts: [string, string][], // field name - field content pairs
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
  render: SearchUiRenderOptions,
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
      if (match[1].length > 0) {
        // For non whitespace tokenized languages, need to backtrack to allow capturing consecutive terms
        termRegex.lastIndex = lastTermPositions[matchedQueryTermIdx];
      }

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
        result.push(createElement('span', { class: 'morsels-ellipsis' }));
        result.push(str.substring(pos - BODY_SERP_BOUND, pos));
        result.push(render.resultsRenderOpts.highlightRender(createElement, render.opts, term));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      } else {
        result.pop();
        result.push(str.substring(prevHighlightEndPos, pos));
        result.push(render.resultsRenderOpts.highlightRender(createElement, render.opts, term));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      }
      prevHighlightEndPos = highlightEndPos;
    }
    result.push(createElement('span', { class: 'morsels-ellipsis' }));

    return { result, numberTermsMatched: lastNumberMatchedTerms };
  }

  let lastIncludedHeading = -1;
  let bestBodyMatch: FinalMatchResult = { result: undefined, numberTermsMatched: 0 };
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

    // Find a new heading this text is under
    let i = itemIdx - 1;
    for (; i > lastIncludedHeading; i -= 1) {
      if (texts[i][0] === 'heading') {
        lastIncludedHeading = i;

        finalMatchResults.push({
          result: render.resultsRenderOpts.headingBodyRender(
            createElement,
            render.opts,
            texts[i][1],
            result,
            (i - 1 >= 0) && texts[i - 1][0] === 'headingLink' && `${baseUrl}#${texts[i - 1][1]}`,
          ),
          numberTermsMatched,
        });
        break;
      }
    }

    // Insert without heading
    if (!finalMatchResults.length && numberTermsMatched > bestBodyMatch.numberTermsMatched) {
      bestBodyMatch = {
        result: render.resultsRenderOpts.bodyOnlyRender(createElement, render.opts, result),
        numberTermsMatched,
      };
    }
  }

  if (!finalMatchResults.length && bestBodyMatch.numberTermsMatched > 0) {
    finalMatchResults.push(bestBodyMatch);
  }

  return finalMatchResults
    .sort((a, b) => b.numberTermsMatched - a.numberTermsMatched)
    .map((r) => r.result)
    .slice(0, MAX_SERP_HIGHLIGHT_PARTS);
}

function transformJson(
  json: any,
  loaderConfig: any,
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
  renderOptions: SearchUiRenderOptions,
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
    bodies: transformText(fields, sortedQueryTerms, termRegex, baseUrl, renderOptions),
  };
}

/*
 Transforms a html document into field name - field content pairs
 ready for highlighting.
 */

function transformHtml(
  doc: Document,
  loaderConfig: any,
  sortedQueryTerms: string[],
  termRegex: RegExp,
  baseUrl: string,
  renderOptions: SearchUiRenderOptions,
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
        fields[fields.length - 1][1] += el.innerText;
      } else {
        fields.push([fieldName, el.innerText || '']);
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
      fields, sortedQueryTerms, termRegex, baseUrl, renderOptions,
    ),
  };
}

/*
 Corrected / "also searched for..." terms
 */

function displayTermInfo(queryParts: QueryPart[], render: SearchUiRenderOptions): HTMLElement[] {
  const misspelledTerms: string[] = [];
  const correctedTerms: string[] = [];
  let expandedTerms: string[] = [];

  queryParts.forEach((queryPart) => {
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
      expandedTerms = queryPart.terms;
    }
  });

  return render.termInfoRender(createElement, render.opts, misspelledTerms, correctedTerms, expandedTerms);
}

/*
 Main transform function
 */

async function singleResultRender(
  result: Result,
  options: SearchUiOptions,
  configs: MorselsConfig,
  hasStoredContentField: FieldInfo,
  query: Query,
  termRegex: RegExp,
) {
  const { loaderConfigs } = configs.indexingConfig;

  const fields = result.getStorageWithFieldNames();
  const relativeFpField = fields.find((v) => v[0] === RELATIVE_LINK_FIELD_NAME);
  const relativeLink = (relativeFpField && relativeFpField[1]) || '';
  const hasSourceFilesUrl = typeof options.sourceFilesUrl === 'string';
  const fullLink = hasSourceFilesUrl ? `${options.sourceFilesUrl}/${relativeLink}` : undefined;
  const titleField = fields.find((v) => v[0] === 'title');
  let resultTitle = (titleField && titleField[1]) || relativeLink;

  let resultHeadingsAndTexts: (string | HTMLElement)[];
  if (hasStoredContentField) {
    resultHeadingsAndTexts = transformText(
      fields.filter((v) => v[0] !== RELATIVE_LINK_FIELD_NAME && v[0] !== 'title'),
      query.searchedTerms,
      termRegex,
      relativeLink,
      options.render,
    );
  } else if (!relativeFpField || !hasSourceFilesUrl) {
    // Unable to retrieve and load from source file
    resultHeadingsAndTexts = [];
  } else if (fullLink.endsWith('.html') && loaderConfigs.HtmlLoader) {
    const asText = await (await fetch(fullLink)).text();
    const doc = domParser.parseFromString(asText, 'text/html');

    const { title: newTitle, bodies: newHeadingsAndTexts } = transformHtml(
      doc, loaderConfigs.HtmlLoader, query.searchedTerms, termRegex, relativeLink, options.render,
    );
    resultTitle = newTitle || resultTitle;
    resultHeadingsAndTexts = newHeadingsAndTexts;
  } else {
    const fullLinkUrl = new URL(fullLink);
    if (fullLinkUrl.pathname.endsWith('.json') && loaderConfigs.JsonLoader) {
      const asJson = await (await fetch(fullLink)).json();

      const { title: newTitle, bodies: newBodies } = transformJson(
        fullLinkUrl.hash ? asJson[fullLinkUrl.hash.substring(1)] : asJson,
        loaderConfigs.JsonLoader,
        query.searchedTerms, termRegex, relativeLink, options.render,
      );
      resultTitle = newTitle || resultTitle;
      resultHeadingsAndTexts = newBodies;
    }
  }

  return options.render.resultsRenderOpts.listItemRender(
    createElement,
    options.render.opts,
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
  const termRegex = new RegExp(
    `(^|\\W)(${query.searchedTerms.map((t) => `(${escapeRegex(t)})`).join('|')})(?=\\W|$)`,
    'gi',
  );

  const hasStoredContentField = config.fieldInfos.find((info) => info.do_store
      && (info.name === 'body' || info.name === 'title' || info.name === 'heading'));

  return Promise.all(results.map(
    (result) => singleResultRender(result, options, config, hasStoredContentField, query, termRegex),
  ));
}

export default async function transformResults(
  query: Query,
  config: MorselsConfig,
  isFirst: boolean,
  container: HTMLElement,
  options: SearchUiOptions,
): Promise<void> {
  const loader = options.render.loadingIndicatorRender(createElement, options.render.opts);
  if (!isFirst) {
    container.appendChild(loader);
  }

  const fragment = document.createDocumentFragment();
  const termInfoEls = isFirst ? displayTermInfo(query.queryParts, options.render) : [];
  termInfoEls.forEach((el) => fragment.appendChild(el));

  // let now = performance.now();

  const results = await query.retrieve(options.render.resultsRenderOpts.resultsPerPage);

  // console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);
  // now = performance.now();

  const resultsEls = await options.render.resultsRender(createElement, options, config, results, query);

  if (resultsEls.length) {
    resultsEls.forEach((el) => fragment.appendChild(el));
  } else if (isFirst) {
    fragment.appendChild(options.render.noResultsRender(createElement, options.render.opts));
  }
  const sentinel = fragment.lastElementChild;

  if (isFirst) {
    container.innerHTML = '';
    container.appendChild(fragment);
  } else {
    loader.replaceWith(fragment);
  }

  // console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  let firstRun = true;
  const iObserver = new IntersectionObserver(async (entries, observer) => {
    if (firstRun || !entries[0].isIntersecting) {
      firstRun = false;
      return;
    }

    observer.unobserve(sentinel);
    sentinel.remove();
    await transformResults(query, config, false, container, options);
  }, { rootMargin: '10px 10px 10px 10px' });

  if (resultsEls.length) {
    iObserver.observe(sentinel);
  }
}
