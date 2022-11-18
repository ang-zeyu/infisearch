import { MatchType, Segment } from './MatchResult';

/*
 Contains the procedure to link 'heading' and 'headingLink' fields to 'body' fields.
*/

// Links the default 'heading', 'headingLink' fields to other texts in the document
export function linkHeadings(
  texts: [string, string][],
  termRegexes: RegExp[],
): Segment[] {
  let lastHeadingMatch: Segment = undefined;
  let lastHeadingLinkIdx = -2;
  let lastHeadingLinkText = '';
  let segments: Segment[] = [];

  for (let pairIdx = 0; pairIdx < texts.length; pairIdx += 1) {
    const [fieldName, fieldText] = texts[pairIdx];
    switch (fieldName) {
      case 'headingLink': {
        lastHeadingLinkIdx = pairIdx;
        lastHeadingLinkText = fieldText;
        break;
      }
      case 'heading': {
        lastHeadingMatch = new Segment(
          MatchType.HEADING_ONLY, fieldText, termRegexes,
          lastHeadingLinkIdx === pairIdx - 1 ? lastHeadingLinkText : undefined,
        );

        // Push a heading-only match in case there are no other matches (unlikely).
        segments.push(lastHeadingMatch);
        break;
      }
      case 'body': {
        const buf = [fieldText];
        let i = pairIdx + 1;
        while (i < texts.length && texts[i][0] === 'body') {
          if (texts[i][1].trim().length) {
            buf.push(texts[i][1]);
          }
          i += 1;
        }
        pairIdx = i - 1;

        const finalMatchResult = new Segment(
          lastHeadingMatch ? MatchType.HEADING_BODY : MatchType.BODY_ONLY,
          buf.join(' ... '), termRegexes,
          lastHeadingMatch?.headingLink, lastHeadingMatch,
        );

        if (lastHeadingMatch) {
          finalMatchResult.numTerms += lastHeadingMatch.numTerms;
        }

        segments.push(finalMatchResult);
        break;
      }
    }
  }

  return segments;
}
