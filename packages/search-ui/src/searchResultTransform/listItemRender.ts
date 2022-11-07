import { ListItemRender, Options } from '../Options';
import { parseURL } from '../utils/url';
import { formatTitle } from './listItemRender/titleFormatter';
import { sortAndLimitResults } from './listItemRender/resultSorter';

// Undocumented option for mdBook
function appendSearchedTerms(
  opts: Options, fullLink: string, searchedTermsJSON: string,
) {
  const { addSearchedTerms } = opts.uiOptions;
  if (addSearchedTerms) {
    const fullLinkUrl = parseURL(fullLink);
    fullLinkUrl.searchParams.append(addSearchedTerms, searchedTermsJSON);
    return fullLinkUrl.toString();
  }
  return fullLink;
}

export const listItemRender: ListItemRender = (
  h,
  opts,
  result,
  query,
) => {
  const {  sourceFilesUrl, useBreadcrumb, maxSubMatches } = opts.uiOptions;

  // -----------------------------------------------------------
  // First sort out the document level info (title, link)

  const fields = result.getKVFields('link', '_relative_fp', 'title', 'h1');

  const title = formatTitle(
    fields.h1 || fields.title || ' ',
    useBreadcrumb,
    fields._relative_fp,
  );

  const hasSourceFilesUrl = typeof sourceFilesUrl === 'string';
  const link = fields.link
    || (hasSourceFilesUrl && fields._relative_fp && `${sourceFilesUrl}${fields._relative_fp}`)
    || '';

  // -----------------------------------------------------------

  // -----------------------------------------------------------
  // Next, get heading, body excerpts (submatches), and format them.
  
  const matchResults = result.getHeadingBodyExcerpts();

  // Limit the number of sub matches
  sortAndLimitResults(matchResults, maxSubMatches);

  const bodies = matchResults.filter(({ type }) => type === 'body').map((res) =>
    res.highlight(),
  );
  const headings = matchResults.filter(({ type }) => type.startsWith('heading')).map((res) => ({
    body: res.highlight(),
    heading: res.heading.highlight(false),
    href: res.headingLink
      ? `${link}#${res.headingLink}`
      : link,
  }));

  // -----------------------------------------------------------

  // -----------------------------------------------------------
  // Construct the HTML output, linking everything together.

  const mainLinkEl = h(
    'a', { class: 'morsels-title-link', role: 'option', tabindex: '-1' },
    h('div', { class: 'morsels-title' }, title),
    ...bodies.map((bodyMatches) => h(
      'div', { class: 'morsels-body' }, ...bodyMatches,
    )),
  );

  if (link) {
    mainLinkEl.setAttribute('href', appendSearchedTerms(opts, link, query._mrlTermsFlattened));
  }

  const subOptions = headings.map(({ body, heading, href }) => {
    const el = h('a',
      {
        class: 'morsels-heading-link',
        role: 'option',
        tabindex: '-1',
      },
      h('div', { class: 'morsels-heading' }, ...heading),
      h('div', { class: 'morsels-body' }, ...body));
    if (href) {
      el.setAttribute('href', appendSearchedTerms(opts, href, query._mrlTermsFlattened));
    }
    return el;
  });

  return Promise.resolve(h(
    'div', { class: 'morsels-list-item', role: 'group', 'aria-label': title },
    mainLinkEl, ...subOptions,
  ));

  // -----------------------------------------------------------
};
