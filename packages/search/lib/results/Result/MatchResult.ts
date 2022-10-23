import h from '../../utils/dom';

function createEllipses() {
  return h('span', { class: 'morsels-ellipses' }, ' ... ');
}

function highlightRender(text: string) {
  return h('mark', { class: 'morsels-highlight' }, text);
}
  
// How far left and right from a match to include in the body
const BODY_SERP_BOUND = 40;
// Maximum preview length for items with no highlights found
const PREVIEW_LENGTH = 80;

// Internal use only
export class BaseSegment {
  constructor(
    public readonly text: string,
    /**
     * Position of the match in the string,
     * and length the match produced by the respective regex
     * 
     * window.length gives the number of terms matched
     */
    public readonly window: { pos: number, len: number }[],
    public numTerms: number,
  ) {}
  
  /**
   * Generates the HTML preview of the match result given.
   */
  highlight(addEllipses: boolean = true): (string | HTMLElement)[] {
    const { text, window } = this;
    
    if (!window.some(({ pos }) => pos >= 0)) {
      if (addEllipses) {
        const preview = text.trimStart().substring(0, PREVIEW_LENGTH);
        return [
          preview.length === PREVIEW_LENGTH ? preview.replace(/\w+$/, '') : preview,
          createEllipses(),
        ];
      } else {
        return [text];
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
        const textToRight = text.substring(pos - BODY_SERP_BOUND, pos);
        result.push(
          textToRight.length === BODY_SERP_BOUND
            ? textToRight.replace(/^\w+/, '')
            : textToRight,
        );
        result.push(highlightRender(text.substring(pos, highlightEndPos)));
      } else if (pos >= prevHighlightEndPos) {
        result.pop();
        result.push(text.substring(prevHighlightEndPos, pos));
        result.push(highlightRender(text.substring(pos, highlightEndPos)));
      } else {
        // Intersecting matches
        if (highlightEndPos > prevHighlightEndPos) {
          result.pop();
          const previousHighlight = result[result.length - 1] as HTMLElement;
          previousHighlight.textContent += text.substring(prevHighlightEndPos, highlightEndPos);
        } else {
          // The highlight is already contained within the previous highlight
          continue;
        }
      }
      
      const textToLeft = text.substring(highlightEndPos, highlightEndPos + BODY_SERP_BOUND);
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
}
