import escapeStringRegexp from 'escape-string-regexp';
import { Query } from '@morsels/search-lib';
import { FieldInfo, MorselsConfig } from '@morsels/search-lib/lib/results/Config';
import Result from '@morsels/search-lib/lib/results/Result';
import { Options, UiMode } from './Options';
import createElement, { CreateElement, createInvisibleLoadingIndicator, MISC_INFO_ID } from './utils/dom';
import { parseURL } from './utils/url';
import { InputState } from './utils/input';
import { transformHtml, transformJson, transformText } from './searchResultTransform/transform';
import { QueryPart } from '@morsels/search-lib/lib/parser/queryParser';

const domParser = new DOMParser();

const RELATIVE_LINK_FIELD_NAME = '_relative_fp';


/**
 * Determines from where (source files / field stores) to retrieve the document's fields.
 * Then calls one of the transformXx variants above.
 */
async function singleResultRender(
  result: Result,
  options: Options,
  configs: MorselsConfig,
  hasStoredContentField: FieldInfo,
  searchedTermsJSON: string,
  termRegexes: RegExp[],
) {
  const { loaderConfigs } = configs.indexingConfig;

  const fields = result.getFields();

  let link: string;
  let relativeLink: string;
  let resultTitle: string;
  let encounteredH1 = false;
  for (const fieldNameAndField of fields) {
    const [fieldName, fieldText] = fieldNameAndField;
    switch (fieldName) {
      case 'link':
        link = link || fieldText;
        break;
      case RELATIVE_LINK_FIELD_NAME:
        relativeLink = relativeLink || fieldText;
        break;
      case 'title':
        resultTitle = resultTitle || fieldText;
        break;
      case 'h1':
        if (!encounteredH1) {
          resultTitle = fieldText;
          encounteredH1 = true;
        }
        break;
      case 'body':
    }
    if (link && relativeLink && resultTitle && encounteredH1) {
      break;
    }
  }

  const {
    useBreadcrumb,
    sourceFilesUrl,
    resultsRenderOpts: { addSearchedTerms, listItemRender },
  } = options.uiOptions;

  const hasSourceFilesUrl = typeof sourceFilesUrl === 'string';
  const fullLink = link
    || (hasSourceFilesUrl && relativeLink && `${sourceFilesUrl}${relativeLink}`)
    || '';

  if (!resultTitle || useBreadcrumb) {
    if (relativeLink) {
      // HTML files: remove the extension
      // PDF: <...breadcumbs...> (PDF)

      const breadCrumbed = relativeLink.split('/')
        .map((component) => {
          /*
           Separate on spaces, underscores, dashes.
           Then assume each sub-component is in camelCase,
           and try to convert to title case.
          */
          return component.split(/[\s_-]+/g)
            .map((text) => text.replace(/([a-z])([A-Z])/g, '$1 $2'))
            .map((text) => text.charAt(0).toUpperCase() + text.slice(1))
            .join(' ');
        })
        .join(' » ');
      const breadCrumbsAndExt = breadCrumbed.split('.');

      let ext = breadCrumbsAndExt.pop().toUpperCase();
      if (ext === 'HTML') {
        ext = '';
      } else if (ext === 'PDF') {
        ext = ' (PDF)';
      } else {
        ext = '.' + ext;
      }

      resultTitle = breadCrumbsAndExt.join('.') + ext;
    } else {
      resultTitle = link;
    }
  }

  let linkToAttach = fullLink;
  if (addSearchedTerms && fullLink) {
    const fullLinkUrl = parseURL(fullLink);
    fullLinkUrl.searchParams.append(
      addSearchedTerms,
      searchedTermsJSON,
    );
    linkToAttach = fullLinkUrl.toString();
  }

  let resultHeadingsAndTexts: (string | HTMLElement)[] = [];
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
    } /* else {
      // CSV / PDF source file generation. Unsupported.
    } */
  }

  return listItemRender(
    createElement,
    options,
    searchedTermsJSON,
    fullLink,
    resultTitle,
    resultHeadingsAndTexts,
    fields,
  );
}

function getSearchedTerms(queryParts: QueryPart[], result: string[][], notContext: boolean) {
  for (const queryPart of queryParts) {
    const currNotContext = (queryPart.isSubtracted || queryPart.isInverted)
      ? !notContext
      : notContext;

    if (queryPart.termsSearched) {
      if (currNotContext) {
        result.push([...queryPart.termsSearched]);
      }
    } else if (queryPart.children) {
      getSearchedTerms(
        queryPart.children,
        result,
        currNotContext,
      );
    }
  }
}

export function resultsRender(
  h: CreateElement,
  options: Options,
  config: MorselsConfig,
  results: Result[],
  query: Query,
): Promise<HTMLElement[]> {
  const termRegexes: RegExp[] = [];

  const searchedTerms: string[][] = [];
  getSearchedTerms(query.queryParts, searchedTerms, true);

  const searchedTermsFlat: string[] = [];
  for (const innerTerms of searchedTerms) {
    const innerTermsJoined = innerTerms
      .map(t => {
        searchedTermsFlat.push(t);
        return escapeStringRegexp(t);
      })
      .sort((a, b) => b.length - a.length)
      .join('|');

    // A little hardcoded, not so pretty but gets the job done for now
    if (config.langConfig.lang === 'latin') {
      const nonEndBoundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})(\\w*?)(?=\\W|$)`, 'gi');
      termRegexes.push(nonEndBoundariedRegex);
    } else {
      const boundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})((?=\\W|$))`, 'gi');
      termRegexes.push(boundariedRegex);
    }
  }

  const hasStoredContentField = config.fieldInfos.find((info) => info.do_store
      && (info.name === 'body' || info.name === 'title' || info.name === 'heading'));

  return Promise.all(results.map(
    (result) => singleResultRender(
      result, options, config, hasStoredContentField, JSON.stringify(searchedTermsFlat), termRegexes,
    ),
  ));
}

/**
 * @returns Whether the results were computed and displayed, or pre-emptively disrupted by a new query
 */
export default async function loadQueryResults(
  inputState: InputState,
  query: Query,
  config: MorselsConfig,
  isFirst: boolean,
  container: HTMLElement,
  options: Options,
): Promise<boolean> {
  if (inputState._mrlNextAction) {
    // If a new query interrupts the current one
    return false;
  }

  const {
    loadingIndicatorRender,
    headerRender,
    resultsPerPage,
    resultsRender: renderResults,
    mode,
  } = options.uiOptions;

  const bottomLoader = loadingIndicatorRender(createElement, options, false, true);
  if (!isFirst) {
    container.appendChild(bottomLoader);
  }

  if (inputState._mrlLastElObserver) {
    inputState._mrlLastElObserver.disconnect();
  }

  const fragment = document.createDocumentFragment();
  if (isFirst) {
    const miscInfo = headerRender(createElement, options, query);
    miscInfo.setAttribute(MISC_INFO_ID, 'true');
    fragment.appendChild(miscInfo);
  }

  //let now = performance.now();

  const results = await query.getNextN(resultsPerPage);

  //console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);
  //now = performance.now();

  if (inputState._mrlNextAction) {
    // If a new query interrupts the current one
    return false;
  }

  const resultsEls = await renderResults(
    createElement, options, config, results, query,
  );

  if (inputState._mrlNextAction) {
    // If a new query interrupts the current one
    return false;
  }

  resultsEls.forEach((el) => fragment.appendChild(el));
  const sentinel = fragment.lastElementChild;

  if (isFirst) {
    container.innerHTML = '';
    inputState._mrlLoader = createInvisibleLoadingIndicator();
    container.append(inputState._mrlLoader);
    container.append(fragment);
  } else {
    bottomLoader.replaceWith(fragment);
  }

  //console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  if (resultsEls.length) {
    const root = mode === UiMode.Target ? null : container;

    inputState._mrlLastElObserver = new IntersectionObserver(async ([entry], observer) => {
      if (!entry.isIntersecting) {
        return;
      }
  
      observer.unobserve(sentinel);
      await loadQueryResults(inputState, query, config, false, container, options);
    }, { root, rootMargin: '150px 0px' });

    inputState._mrlLastElObserver.observe(sentinel);
  }

  return true;
}
