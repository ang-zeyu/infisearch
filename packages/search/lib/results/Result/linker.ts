import { MatchType, Segment } from './MatchResult';

/*
 Contains the procedure to link 'heading' and 'headingLink' fields to content fields.
*/

// Links the default 'heading', 'headingLink' fields to other texts in the document
export function linkHeadings(
  texts: [string, string][],
  termRegexes: RegExp[],
  contentFields: string[],
): Segment[] {
  let lastHeadingMatch: Segment = undefined;
  let lastHeadingLinkIdx = -2;
  let lastHeadingLinkText = '';
  let segments: Segment[] = [];

  for (let pairIdx = 0; pairIdx < texts.length; pairIdx += 1) {
    const [fieldName, fieldText] = texts[pairIdx];
    if (fieldName === 'headingLink') {
      lastHeadingLinkIdx = pairIdx;
      lastHeadingLinkText = fieldText;
    } else if (fieldName === 'heading') {
      lastHeadingMatch = new Segment(
        MatchType.HEADING_ONLY, fieldText, termRegexes,
        lastHeadingLinkIdx === pairIdx - 1 ? lastHeadingLinkText : undefined,
      );

      // Push a heading-only match in case there are no other matches (unlikely).
      segments.push(lastHeadingMatch);
    } else if (contentFields.includes(fieldName)) {
      const buf = [fieldText];
      let i = pairIdx + 1;
      while (i < texts.length && contentFields.includes(texts[i][0])) {
        if (texts[i][1].trim().length) {
          buf.push(texts[i][1]);
        }
        i += 1;
      }
      pairIdx = i - 1;

      const finalMatchResult = new Segment(
        lastHeadingMatch ? MatchType.CONTENT_AND_HEADING : MatchType.CONTENT,
        buf.join(' ... '), termRegexes,
        lastHeadingMatch?.headingLink, lastHeadingMatch,
      );

      if (lastHeadingMatch) {
        finalMatchResult.numTerms += lastHeadingMatch.numTerms;
      }

      segments.push(finalMatchResult);
    }
  }

  return segments;
}
