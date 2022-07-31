import { Options } from '../Options';
import createElement from '../utils/dom';

export interface MatchResult {
  _mrlStr: string,
  /**
     * Position of the match in the string,
     * and length the match produced by the respective regex
     */
  _mrlWindow: { pos: number, len: number }[],
  _mrlNumTerms: number,
}
  
/**
   * Generates the closest window (while preferring longer regex matches) in a given string.
   */
export function getBestMatchResult(_mrlStr: string, termRegexes: RegExp[]): MatchResult {
  // Get all matches first
  const matches = termRegexes.map(r => Array.from(_mrlStr.matchAll(r)));
  if (!matches.some(innerMatches => innerMatches.length)) {
    return {
      _mrlStr,
      _mrlWindow: [],
      _mrlNumTerms: 0,
    };
  }
  
  let lastClosestRegexPositions = termRegexes.map(() => -1);
  let lastClosestWindowLen = 10000000;
  let lastClosestRegexLengths = termRegexes.map(() => 0);
  
  // Next match index of each RegExp's array inside matches
  const matchIndices = matches.map(() => 0);
  const hasFinished =  matches.map((innerMatches) => !innerMatches.length);
  
  // Local to the while (true) loop; To avoid .map and reallocating
  const matchPositions = matches.map(() => -1);
  
  while (true) {
    let lowestMatchPos = 10000000;
    let lowestMatchPosExclFinished = 10000000;
    let lowestMatchIndex = -1;
    let highestMatchPos = 0;

    for (let regexIdx = 0; regexIdx < matchIndices.length; regexIdx++) {
      const match = matches[regexIdx][matchIndices[regexIdx]];
      if (!match) {
        // No matches at all for this regex in this str
        continue;
      }
  
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
    if (windowLen < lastClosestWindowLen) {
      lastClosestWindowLen = windowLen;
      lastClosestRegexPositions = [...matchPositions];
      lastClosestRegexLengths = matchIndices.map((i, idx) => matches[idx][i] && (
        matches[idx][i][2].length + matches[idx][i][3].length
      ));
    }
  
    // Forward the match with the smallest position
    matchIndices[lowestMatchIndex] += 1;
    if (matchIndices[lowestMatchIndex] >= matches[lowestMatchIndex].length) {
      hasFinished[lowestMatchIndex] = true;
      matchIndices[lowestMatchIndex] -= 1;
      if (hasFinished.every(finished => finished)) {
        break;
      }
    }
  }
  
  const window = lastClosestRegexPositions
    .map((pos, idx) => ({ pos, len: lastClosestRegexLengths[idx] }))
    .filter((pair) => pair.pos >= 0)
    .sort((a, b) => a.pos - b.pos);
  const numTerms = window.length;
  return { _mrlStr, _mrlWindow: window, _mrlNumTerms: numTerms };
}
  
function createEllipses() {
  return createElement('span', { class: 'morsels-ellipsis', 'aria-label': 'ellipses' }, ' ... ');
}

// How far left and right from a match to include in the body
const BODY_SERP_BOUND = 40;
// Maximum preview length for items with no highlights found
const PREVIEW_LENGTH = 80;

/**
 * Generates the HTML preview of the match result given.
 */
export function highlightMatchResult(
  matchResult: MatchResult,
  addEllipses: boolean,
  options: Options,
): (string | HTMLElement)[] {
  const { highlightRender } = options.uiOptions.resultsRenderOpts;
  const { _mrlStr: str, _mrlWindow: window } = matchResult;
  
  if (!window.some(({ pos }) => pos >= 0)) {
    if (addEllipses) {
      const preview = str.trimStart().substring(0, PREVIEW_LENGTH);
      return [
        preview.length === PREVIEW_LENGTH ? preview.replace(/\w+$/, '') : preview,
        createEllipses(),
      ];
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
      const textToRight = str.substring(pos - BODY_SERP_BOUND, pos);
      result.push(
        textToRight.length === BODY_SERP_BOUND
          ? textToRight.replace(/^\w+/, '')
          : textToRight,
      );
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
    
    const textToLeft = str.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND);
    result.push(
      textToLeft.length === BODY_SERP_BOUND
        ? textToLeft.replace(/\w+$/, '')
        : textToLeft,
    );
  
    prevHighlightEndPos = highlightEndPos;
  }
  
  if (addEllipses) {
    result.push(createEllipses());
  }
  
  return result;
}