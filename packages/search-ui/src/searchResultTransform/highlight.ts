import { SearchUiOptions } from '../SearchUiOptions';
import createElement from '../utils/dom';

// How far left and right from a match to include in the body
const BODY_SERP_BOUND = 40;

export interface MatchResult {
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
export function getBestMatchResult(str: string, termRegexes: RegExp[]): MatchResult {
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
export function highlightMatchResult(
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