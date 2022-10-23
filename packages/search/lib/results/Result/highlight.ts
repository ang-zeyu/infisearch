import { BaseSegment } from './MatchResult';

/**
   * Generates the closest window (while preferring longer regex matches) in a given string.
   */
export function getBestMatchResult(text: string, termRegexes: RegExp[]): BaseSegment {
  // Get all matches first
  const matches = termRegexes.map(r => Array.from(text.matchAll(r)));
  if (!matches.some(innerMatches => innerMatches.length)) {
    return new BaseSegment(text, [], 0);
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
  return new BaseSegment(text, window, window.length);
}
