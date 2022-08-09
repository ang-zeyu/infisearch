import createElement from '../utils/dom';
import { Options } from '../Options';
import { getBestMatchResult, highlightMatchResult, MatchResult } from './highlight';

interface ProcessedMatchResult extends MatchResult {
  _mrlPairIdx: number,
  _mrlHeadingMatch?: MatchResult,
  _mrlHeadingLink?: string,
}
  
/**
   * Finds, cuts, and highlights the best matching excerpt of 'heading' and 'body' fields
   * @param texts array of ['field name', 'field content'] pairs
   */
export function transformText(
  texts: [string, string][],
  termRegexes: RegExp[],
  baseUrl: string,
  options: Options,
): (string | HTMLElement)[] {
  const { maxSubMatches, resultsRenderOpts } = options.uiOptions;
  const {
    bodyOnlyRender,
    headingBodyRender,
  } = resultsRenderOpts;
  
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
        lastHeadingMatch._mrlPairIdx = pairIdx;
        lastHeadingMatch._mrlHeadingLink = lastHeadingLinkIdx === lastHeadingMatch._mrlPairIdx - 1
          ? lastHeadingLinkText
          : '';
          
        // Push a heading-only match in case there are no other matches (unlikely).
        matchResults.push({
          _mrlStr: '',
          _mrlWindow: [],
          _mrlNumTerms: -2000, // even less preferable than body-only matches
          _mrlHeadingMatch: lastHeadingMatch,
          _mrlHeadingLink: lastHeadingMatch._mrlHeadingLink,
          _mrlPairIdx: pairIdx,
        });
        break;
      }
      case 'body': {
        const finalMatchResult = getBestMatchResult(fieldText, termRegexes) as ProcessedMatchResult;
        if (lastHeadingMatch) {
          // Link up body matches with headings, headingLinks if any
          finalMatchResult._mrlHeadingMatch = lastHeadingMatch;
          finalMatchResult._mrlHeadingLink = lastHeadingMatch._mrlHeadingLink;
          finalMatchResult._mrlNumTerms += lastHeadingMatch._mrlNumTerms;
        } else {
          // body-only match, add an offset to prefer heading-body matches
          finalMatchResult._mrlNumTerms -= 1000;
        }
        matchResults.push(finalMatchResult);
        break;
      }
    }
  }
  
  matchResults.sort((a, b) => {
    return a._mrlNumTerms === 0 && b._mrlNumTerms === 0
    // If there are 0 terms matched for both matches, prefer "longer" snippets
      ? b._mrlStr.length - a._mrlStr.length
      : b._mrlNumTerms - a._mrlNumTerms;
  });
  
  const matches: ProcessedMatchResult[] = [];
  const maxMatches = Math.min(matchResults.length, maxSubMatches);
  for (let i = 0; i < maxMatches; i += 1) {
    if (matchResults[i]._mrlNumTerms !== matchResults[0]._mrlNumTerms) {
      break;
    }
    matches.push(matchResults[i]);
  }
  
  return matches.map((finalMatchResult) => {
    const bodyHighlights = highlightMatchResult(finalMatchResult, true, options);
    if (finalMatchResult._mrlHeadingMatch) {
      const highlightedHeadings = highlightMatchResult(finalMatchResult._mrlHeadingMatch, false, options);
      const headingHighlights = highlightedHeadings.length
        ? highlightedHeadings
        : [finalMatchResult._mrlHeadingMatch._mrlStr];
      const href = finalMatchResult._mrlHeadingLink && `${baseUrl}#${finalMatchResult._mrlHeadingLink}`;
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
  
export function transformJson(
  json: any,
  loaderConfig: any,
  termRegexes: RegExp[],
  baseUrl: string,
  options: Options,
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
export function transformHtml(
  doc: Document,
  loaderConfig: any,
  termRegexes: RegExp[],
  baseUrl: string,
  options: Options,
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
  
  let title = '';
  let encounteredH1 = false;
  for (const fieldNameAndField of fields) {
    const [fieldName, fieldText] = fieldNameAndField;
    if (fieldName === 'title') {
      title = title || fieldText;
    } else if (fieldName === 'h1' && !encounteredH1) {
      title = fieldText;
      encounteredH1 = true;
    }

    if (title && encounteredH1) {
      break;
    }
  }
  
  return {
    title,
    bodies: transformText(
      fields, termRegexes, baseUrl, options,
    ),
  };
}
