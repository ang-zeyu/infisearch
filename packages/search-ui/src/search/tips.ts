import { computePosition, flip } from '@floating-ui/dom';
import { InfiConfig } from '@infisearch/search-lib/lib/results/Config';
import h from '@infisearch/search-lib/lib/utils/dom';
import { UiOptions } from '../Options';

export default function createTipButton(
  opts: UiOptions,
  cfg: InfiConfig,
): HTMLElement | string {
  const { tip, translations } = opts;

  if (tip === false) {
    return '';
  }

  function wrapInCode(example: string) {
    return h('code', {}, example);
  }

  function createRow(...contents: (string | HTMLElement)[]) {
    return h('tr', { class: 'infi-tip-item' }, ...contents.map((el) => h('td', {}, h('div', {}, el))));
  }

  const tipListBody = h('tbody', {});

  const tipRows = translations.tipRows;

  if (cfg.indexingConfig.withPositions) {
    tipListBody.append(createRow(
      tipRows.searchPhrases,
      wrapInCode(tipRows.exSearchPhrases),
    ));
  }

  tipListBody.append(
    createRow(
      tipRows.requireTerm,
      wrapInCode(tipRows.exRequireTerm),
    ),
    createRow(
      tipRows.excludeTerm,
      wrapInCode(tipRows.exExcludeTerm),
    ),
    createRow(
      tipRows.flipResults,
      wrapInCode(tipRows.exFlipResults),
    ),
    createRow(
      tipRows.groupTerms,
      wrapInCode(tipRows.exGroupTerms),
    ),
    createRow(
      tipRows.searchPrefixes,
      wrapInCode(tipRows.exSearchPrefixes),
    ),
    createRow(
      tipRows.searchSections,
      h('ul', {},
        ...tipRows.exSearchSections.map(t => h('li', {}, wrapInCode(t))),
      ),
    ),
  );

  const tipList = h(
    'table',
    { class: 'infi-tip-table' },
    h(
      'thead',
      { class: 'infi-tip-table-header' },
      h('tr', {}, h('th', { scope: 'col' }, translations.tip), h('th', {}, translations.example)),
    ),
    tipListBody,
  );
  const tipPopup = h(
    'div', { class: 'infi-tip-popup-root' },
    h('div', { class: 'infi-tip-popup' },
      h('div', { class: 'infi-tip-popup-title' }, translations.tipHeader),
      tipList,
    ),
  );

  let shown = false;
  function hide() {
    if (shown) {
      tipPopup.classList.remove('shown');
      shown = false;
    }
  }

  tipPopup.ontransitionend = () => {
    if (!shown) {
      tipPopup.style.transform = 'scale(0)';
    }
  };

  const tipContainer = h(
    'div', { class: 'infi-tip-root', tabindex: '0' },
    h('span', { class: 'infi-tip-icon' }, '?'),
    tipPopup,
  );

  function show() {
    shown = true;
    computePosition(tipContainer, tipPopup, {
      placement: 'top-end',
      middleware: [
        flip({
          crossAxis: false,
          flipAlignment: true,
          padding: 10,
          boundary: document.body,
        }),
      ],
    }).then(({ x, y }) => {
      Object.assign(tipPopup.style, {
        left: `${x}px`,
        top: `${y}px`,
        transform: 'scale(1)',
      });
      tipPopup.classList.add('shown');
    });
  }

  tipContainer.onmouseover = show;
  tipContainer.onfocus = show;
  tipContainer.onmouseleave = hide;
  tipContainer.onblur = hide;

  return tipContainer;
}
