import { Match, Options } from '../Options';
import { getBestMatchResult, highlightMatchResult, MatchResult } from './highlight';

enum MatchType {
  HEADING_BODY = 2,
  BODY_ONLY = 1,
  HEADING_ONLY = 0,
}

interface ProcessedMatchResult extends MatchResult {
  _mrlPairIdx: number,
  _mrlHeadingMatch?: MatchResult,
  _mrlHeadingLink?: string,
  _mrlType: MatchType,
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
): Match[] {
  const { maxSubMatches } = options.uiOptions;
  
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
        lastHeadingMatch._mrlHeadingLink = lastHeadingLinkIdx === pairIdx - 1
          ? lastHeadingLinkText
          : '';
          
        // Push a heading-only match in case there are no other matches (unlikely).
        matchResults.push({
          _mrlStr: '',
          _mrlWindow: [],
          _mrlNumTerms: 0,
          _mrlHeadingMatch: lastHeadingMatch,
          _mrlHeadingLink: lastHeadingMatch._mrlHeadingLink,
          _mrlPairIdx: pairIdx,
          _mrlType: MatchType.HEADING_ONLY,
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
          finalMatchResult._mrlType = MatchType.HEADING_BODY;
        } else {
          finalMatchResult._mrlType = MatchType.BODY_ONLY;
        }
        matchResults.push(finalMatchResult);
        break;
      }
    }
  }
  
  matchResults.sort((a, b) => {
    if (a._mrlNumTerms === 0 && b._mrlNumTerms === 0) {
      // If there are 0 terms matched for both matches, prefer "longer" snippets
      return b._mrlStr.length - a._mrlStr.length;
    }
    return a._mrlNumTerms === b._mrlNumTerms
      ? b._mrlType - a._mrlType
      : b._mrlNumTerms - a._mrlNumTerms;
  });
  
  const matches: ProcessedMatchResult[] = [];
  const maxMatches = Math.min(matchResults.length, maxSubMatches);
  for (let i = 0; i < maxMatches; i += 1) {
    if (matchResults[i]._mrlNumTerms !== matchResults[0]._mrlNumTerms
      || matchResults[i]._mrlType !== matchResults[0]._mrlType) {
      break;
    }
    matches.push(matchResults[i]);
  }

  return matches.map((finalMatchResult) => {
    const result: Match = {
      bodyMatches: highlightMatchResult(finalMatchResult, true, options),
    };

    if (finalMatchResult._mrlHeadingMatch) {
      const highlightedHeadings = highlightMatchResult(finalMatchResult._mrlHeadingMatch, false, options);
      result.headingMatches = highlightedHeadings.length
        ? highlightedHeadings
        : [finalMatchResult._mrlHeadingMatch._mrlStr];
      result.href = finalMatchResult._mrlHeadingLink
        ? `${baseUrl}#${finalMatchResult._mrlHeadingLink}`
        : baseUrl;
    }

    return result;
  });
}
