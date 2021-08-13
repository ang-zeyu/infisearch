import * as escapeRegex from 'escape-string-regexp';
import { Query } from '@morsels/search-lib';
import { MorselsConfig } from '@morsels/search-lib/lib/results/FieldInfo';
import { QueryPart } from '@morsels/search-lib/lib/parser/queryParser';
import { SearchUiOptions, SearchUiRenderOptions } from './SearchUiOptions';
import createElement from './utils/dom';

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
        result.push(render.highlightRender(createElement, term));
        result.push(str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND));
      } else {
        result.pop();
        result.push(str.substring(prevHighlightEndPos, pos));
        result.push(render.highlightRender(createElement, term));
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
        finalMatchResult.result = render.headingBodyRender(
          createElement,
          texts[i][1],
          result,
          (i - 1 >= 0) && texts[i - 1][0] === 'headingLink' && `${baseUrl}#${texts[i - 1][1]}`,
        );
        break;
      }
    }

    // Insert without heading
    if (!finalMatchResult.result) {
      finalMatchResult.result = render.bodyOnlyRender(createElement, result);
    }
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

  if (loaderConfig.excludeSelectors) {
    for (const excludeSelector of loaderConfig.exclude_selectors) {
      const nodes = doc.querySelectorAll(excludeSelector);
      for (let i = 0; i < nodes.length; i += 1) {
        nodes[i].remove();
      }
    }
  }

  loaderConfig.selectors = loaderConfig.selectors || [];

  function traverseBody(el: HTMLElement, fieldName: string) {
    for (const selector of loaderConfig.selectors) {
      if (el.matches(selector.selector)) {
        if (selector.attr_map) {
          Object.entries(selector.attr_map).forEach(([attrName, attrFieldName]) => {
            if (el.attributes[attrName]) {
              fields.push([attrFieldName as any, el.attributes[attrName].value]);
            }
          });
        }

        for (let i = 0; i < el.childNodes.length; i += 1) {
          const child = el.childNodes[i];
          if (child.nodeType === Node.ELEMENT_NODE) {
            traverseBody(child as HTMLElement, selector.field_name);
          } else if (child.nodeType === Node.TEXT_NODE && selector.field_name) {
            if (fields.length && fields[fields.length - 1][0] === selector.field_name) {
              fields[fields.length - 1][1] += (child as Text).data;
            } else {
              fields.push([selector.field_name, (child as Text).data]);
            }
          }
        }
        return;
      }
    }

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
  }

  traverseBody(doc.documentElement, undefined);

  const titleIdx = fields.findIndex((pair) => pair[0] === 'title');
  const title = titleIdx === -1 ? undefined : fields.splice(titleIdx, 1)[0][1];

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

  return render.termInfoRender(createElement, misspelledTerms, correctedTerms, expandedTerms);
}

/*
 Main transform function
 */

export default async function transformResults(
  query: Query,
  config: MorselsConfig,
  isFirst: boolean,
  container: HTMLElement,
  options: SearchUiOptions,
): Promise<void> {
  const termRegex = new RegExp(
    `(^|\\W)(${query.searchedTerms.map((t) => `(${escapeRegex(t)})`).join('|')})(?=\\W|$)`,
    'gi',
  );

  const loader = options.render.loadingIndicatorRender(createElement);
  if (!isFirst) {
    container.appendChild(loader);
  }

  const fragment = document.createDocumentFragment();
  const termInfoEls = isFirst ? displayTermInfo(query.queryParts, options.render) : [];
  termInfoEls.forEach((el) => fragment.appendChild(el));

  let now = performance.now();

  const results = await query.retrieve(options.resultsPerPage);

  console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);
  now = performance.now();

  const { fieldInfos, indexingConfig } = config;
  const { loaderConfigs } = indexingConfig;
  const hasStoredContentField = fieldInfos.find((info) => info.do_store
      && (info.name === 'body' || info.name === 'title' || info.name === 'heading'));

  const resultsEls = await Promise.all(results.map(async (result) => {
    console.log(result);

    const fields = result.getStorageWithFieldNames();
    const linkField = fields.find((v) => v[0] === 'link');
    const relativeLink = (linkField && linkField[1]) || '';
    const fullLink = options.sourceFilesUrl ? `${options.sourceFilesUrl}/${relativeLink}` : undefined;
    const titleField = fields.find((v) => v[0] === 'title');
    let resultTitle = (titleField && titleField[1]) || relativeLink;

    let resultHeadingsAndTexts: (string | HTMLElement)[];
    if (hasStoredContentField) {
      resultHeadingsAndTexts = transformText(
        fields.filter((v) => v[0] !== 'link' && v[0] !== 'title'),
        query.searchedTerms,
        termRegex,
        relativeLink,
        options.render,
      );
    } else if (!linkField || !options.sourceFilesUrl) {
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
    } else if (fullLink.endsWith('.json') && loaderConfigs.JsonLoader) {
      const asJson = await (await fetch(fullLink)).json();

      const { title: newTitle, bodies: newBodies } = transformJson(
        asJson,
        loaderConfigs.JsonLoader,
        query.searchedTerms, termRegex, relativeLink, options.render,
      );
      resultTitle = newTitle || resultTitle;
      resultHeadingsAndTexts = newBodies;
    }

    return options.render.listItemRender(
      createElement,
      fullLink,
      resultTitle,
      resultHeadingsAndTexts,
      fields,
    );
  }));
  if (resultsEls.length) {
    resultsEls.forEach((el) => fragment.appendChild(el));
  } else if (isFirst) {
    fragment.appendChild(options.render.noResultsRender(createElement));
  }
  const sentinel = fragment.lastElementChild;

  if (isFirst) {
    container.innerHTML = '';
    container.appendChild(fragment);
  } else {
    loader.replaceWith(fragment);
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
    await transformResults(query, config, false, container, options);
  }, { rootMargin: '10px 10px 10px 10px' });
  iObserver.observe(sentinel);
}
