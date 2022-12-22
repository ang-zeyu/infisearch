import { ListItemRender } from '../Options';
import { parseURL } from '../utils/url';
import { formatTitle } from './listItemRender/titleFormatter';
import { sortAndLimitResults } from './listItemRender/resultSorter';
import { createEllipses } from '@infisearch/search-lib/lib/results/Result/MatchResult';

// Undocumented option for mdBook to attach query terms to the URL
function appendSearchedTerms(
  fullLink: string,
  searchedTermsParam: string,
  searchedTermsJSON: string,
) {
  if (searchedTermsParam) {
    const fullLinkUrl = parseURL(fullLink);
    fullLinkUrl.searchParams.append(searchedTermsParam, searchedTermsJSON);
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
  const {
    sourceFilesUrl,
    useBreadcrumb,
    maxSubMatches,
    searchedTermsParam,
    onLinkClick,
    contentFields,
  } = opts.uiOptions;

  // -----------------------------------------------------------
  // First sort out the document level info (title, link)

  const fields = result.getKVFields('link', '_relative_fp', 'title', 'h1');

  const title = formatTitle(
    fields.h1 || fields.title || '',
    useBreadcrumb,
    fields._relative_fp,
  );

  const hasSourceFilesUrl = typeof sourceFilesUrl === 'string';
  const link = fields.link
    || (hasSourceFilesUrl && fields._relative_fp && `${sourceFilesUrl}${fields._relative_fp}`)
    || '';

  // -----------------------------------------------------------

  // -----------------------------------------------------------
  // Next, link headings to contents (submatches), and format them.

  const matchResults = result.linkHeadingsToContents(...contentFields);

  // Limit the number of sub matches
  sortAndLimitResults(matchResults, maxSubMatches);

  const contents = matchResults.filter(({ type }) => type === 'content').map((res) =>
    res.highlight(),
  );
  const headingsAndContents = matchResults.filter(({ type }) => type.startsWith('heading')).map((res) => ({
    content: res.heading ? res.highlight() : [createEllipses()],
    heading: res.heading ? res.heading.highlight(false) : res.highlight(),
    href: res.headingLink
      ? `${link}#${res.headingLink}`
      : link,
  }));

  // -----------------------------------------------------------

  // -----------------------------------------------------------
  // Construct the HTML output, linking everything together.

  const mainLinkEl = h(
    'a', { class: 'infi-title-link', role: 'option', tabindex: '-1' },
    h('div', { class: 'infi-title' }, title),
    ...contents.map((contentMatches) => h(
      'div', { class: 'infi-body' }, ...contentMatches,
    )),
  );

  if (link) {
    mainLinkEl.setAttribute('href', appendSearchedTerms(link, searchedTermsParam, query._mrlTermsFlattened));
    mainLinkEl.onclick = onLinkClick;
  }

  const subOptions = headingsAndContents.map(({ content, heading, href }) => {
    const el = h('a',
      {
        class: 'infi-heading-link',
        role: 'option',
        tabindex: '-1',
      },
      h('div', { class: 'infi-heading' }, ...heading),
      h('div', { class: 'infi-body' }, ...content));
    if (href) {
      el.setAttribute('href', appendSearchedTerms(href, searchedTermsParam, query._mrlTermsFlattened));
      el.onclick = onLinkClick;
    }
    return el;
  });

  return Promise.resolve(h(
    'div', { class: 'infi-list-item', role: 'group', 'aria-label': title },
    mainLinkEl, ...subOptions,
  ));

  // -----------------------------------------------------------
};
