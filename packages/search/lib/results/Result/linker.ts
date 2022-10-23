import { getBestMatchResult } from './highlight';
import { BaseSegment } from './MatchResult';

/*
 Contains the procedure to link 'heading' and 'headingLink' fields to 'body' fields.
*/

export enum MatchType {
  HEADING_BODY = 'heading-body',
  BODY_ONLY = 'body',
  HEADING_ONLY = 'heading',
}

export class Segment extends BaseSegment {
  constructor(
    text: string,
    window: { pos: number, len: number }[],
    numTerms: number,
    public heading: Segment,
    public headingLink: string,
    public type: MatchType,
  ) {
    super(text, window, numTerms);
  }
}

// Links the default 'heading', 'headingLink' fields to other texts in the document
export function linkHeadings(
  texts: [string, string][],
  termRegexes: RegExp[],
): Segment[] {
  let lastHeadingMatch: Segment = undefined;
  let lastHeadingLinkIdx = -2;
  let lastHeadingLinkText = '';
  let matchResults: Segment[] = [];

  for (let pairIdx = 0; pairIdx < texts.length; pairIdx += 1) {
    const [fieldName, fieldText] = texts[pairIdx];
    switch (fieldName) {
      case 'headingLink': {
        lastHeadingLinkIdx = pairIdx;
        lastHeadingLinkText = fieldText;
        break;
      }
      case 'heading': {
        lastHeadingMatch = getBestMatchResult(fieldText, termRegexes) as Segment;
        lastHeadingMatch.headingLink = lastHeadingLinkIdx === pairIdx - 1
          ? lastHeadingLinkText
          : '';
            
        // Push a heading-only match in case there are no other matches (unlikely).
        matchResults.push(new Segment(
          '', [], 0,
          lastHeadingMatch, lastHeadingMatch.headingLink, MatchType.HEADING_ONLY,
        ));
        break;
      }
      case 'body': {
        const finalMatchResult = getBestMatchResult(fieldText, termRegexes) as Segment;
        if (lastHeadingMatch) {
          // Link up body matches with headings, headingLinks if any
          finalMatchResult.heading = lastHeadingMatch;
          finalMatchResult.headingLink = lastHeadingMatch.headingLink;
          finalMatchResult.numTerms += lastHeadingMatch.numTerms;
          finalMatchResult.type = MatchType.HEADING_BODY;
        } else {
          finalMatchResult.type = MatchType.BODY_ONLY;
        }
        matchResults.push(finalMatchResult);
        break;
      }
    }
  }

  return matchResults;
}
