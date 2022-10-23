import escapeStringRegexp from 'escape-string-regexp';
import { Query, Searcher } from '@morsels/search-lib';
import { MorselsConfig } from '@morsels/search-lib/lib/results/Config';
import Result from '@morsels/search-lib/lib/results/Result';
import { Options } from './Options';
import createElement, { CreateElement, createInvisibleLoadingIndicator } from './utils/dom';
import { parseURL } from './utils/url';
import { InputState } from './utils/input';
import { transformText } from './searchResultTransform/transform';
import { QueryPart } from '@morsels/search-lib/lib/parser/queryParser';
import { resultSeparator } from './searchResultTransform/repeatedFooter';
import { focusEl } from './utils/keyboard';

const RELATIVE_LINK_FIELD_NAME = '_relative_fp';

async function singleResultRender(
  result: Result,
  options: Options,
  searchedTermsJSON: string,
  termRegexes: RegExp[],
) {
  // Contains [field name, field text] pairs in the order they were indexed
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

  return listItemRender(
    createElement,
    options,
    searchedTermsJSON,
    fullLink,
    resultTitle,
    transformText(
      fields,
      termRegexes,
      linkToAttach,
      options,
    ),
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
  numResultsSoFar: number,
  loadMore: (nResults: number) => Promise<HTMLElement[] | undefined>,
  focusOption: (el: HTMLElement) => void,
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

    if (config.langConfig.lang === 'latin') {
      const nonEndBoundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})(\\w*?)(?=\\W|$)`, 'gi');
      termRegexes.push(nonEndBoundariedRegex);
    } else {
      const boundariedRegex = new RegExp(`(^|\\W|_)(${innerTermsJoined})((?=\\W|$))`, 'gi');
      termRegexes.push(boundariedRegex);
    }
  }

  return Promise.all(results.map(
    (result) => singleResultRender(
      result, options, JSON.stringify(searchedTermsFlat), termRegexes,
    ),
  )).then((resultEls) => {
    resultEls.push(resultSeparator(
      options,
      numResultsSoFar + results.length,
      results.length < options.uiOptions.resultsPerPage,
      loadMore, focusOption, query,
    ));

    return resultEls;
  });
}

/**
 * @returns The rendered result elements, or undefined if pre-emptively disrupted by a new query
 */
export default async function loadQueryResults(
  searcher: Searcher,
  inputState: InputState,
  query: Query,
  resultsToLoad: number,
  numResultsSoFar: number,
  options: Options,
): Promise<HTMLElement[] | undefined> {
  // If a new query interrupts the current one
  if (inputState._mrlNextAction) return;

  const {
    headerRender,
    resultsRender: renderResults,
  } = options.uiOptions;

  // let now = performance.now();

  const results = await query.getNextN(resultsToLoad);

  // console.log(`Search Result Retrieval took ${performance.now() - now} milliseconds`);

  if (inputState._mrlNextAction) return;

  // now = performance.now();

  const inputEl = inputState._mrlInputEl;
  const listContainer = inputState._mrlListContainer;
  const resultsEls = await renderResults(
    createElement,
    options,
    searcher.cfg,
    results,
    query,
    numResultsSoFar,
    (nResults: number) => {
      // inputEl.focus(); -- this wont work. causes keyboard to reshow on mobile
      return loadQueryResults(
        searcher, inputState, query, 
        nResults, numResultsSoFar + results.length, options,
      );
    },
    (el: HTMLElement) => focusEl(
      el, listContainer.querySelector('#morsels-list-selected'), inputEl, listContainer, false,
    ),
  );

  // console.log(`Result transformation took ${performance.now() - now} milliseconds`);

  if (inputState._mrlNextAction) return;

  if (numResultsSoFar) {
    listContainer.append(...resultsEls);
  } else {
    listContainer.innerHTML = '';
    inputState._mrlLoader = createInvisibleLoadingIndicator();
    listContainer.append(
      inputState._mrlLoader,
      headerRender(createElement, options, false, false, query),
      ...resultsEls,
    );
  }

  return resultsEls;
}
