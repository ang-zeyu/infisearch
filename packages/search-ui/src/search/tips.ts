import { computePosition, flip } from '@floating-ui/dom';
import { InfiConfig } from '@infisearch/search-lib/lib/results/Config';
import h from '@infisearch/search-lib/lib/utils/dom';
import { UiOptions } from '../Options';

export default function createTipButton(
  opts: UiOptions,
  cfg: InfiConfig,
): HTMLElement | string {
  if (opts.tip === false) {
    return '';
  }

  function wrapInCode(example: string) {
    return h('code', {}, example);
  }

  function createRow(...contents: (string | HTMLElement)[]) {
    return h('tr', { class: 'infi-tip-item' }, ...contents.map((el) => h('td', {}, h('div', {}, el))));
  }

  const tipListBody = h('tbody', {});

  if (cfg.indexingConfig.withPositions) {
    tipListBody.append(createRow(
      'Search for phrases',
      wrapInCode('"for tomorrow"'),
    ));
  }

  tipListBody.append(
    createRow(
      'Require a term',
      wrapInCode('+sunny weather'),
    ),
    createRow(
      'Exclude a term',
      wrapInCode('-cloudy sunny'),
    ),
    createRow(
      'Flip search results',
      wrapInCode('~rainy'),
    ),
    createRow(
      'Group terms together',
      wrapInCode('~(sunny warm cloudy)'),
    ),
    createRow(
      'Search for prefixes',
      wrapInCode('run*'),
    ),
    createRow(
      'Search only specific sections',
      h('ul', {}, 
        h('li', {}, wrapInCode('title:forecast')),
        h('li', {}, wrapInCode('heading:sunny')),
        h('li', {}, wrapInCode('body:(rainy gloomy)')),
      ),
    ),
  );

  const tipList = h(
    'table',
    { class: 'infi-tip-table' },
    h(
      'thead',
      { class: 'infi-tip-table-header' },
      h('tr', {}, h('th', { scope: 'col' }, 'Tip'), h('th', {}, 'Example')),
    ),
    tipListBody,
  );
  const tipPopup = h(
    'div', { class: 'infi-tip-popup-root' },
    h('div', { class: 'infi-tip-popup' },
      h('div', { class: 'infi-tip-popup-title' }, '🔎 Advanced search tips'),
      tipList,
    ),
  );

  function resetPopupStyle() {
    tipPopup.classList.remove('shown');
  }
  resetPopupStyle();

  const tipContainer = h(
    'div', { class: 'infi-tip-root', tabindex: '0' },
    h('span', { class: 'infi-tip-icon' }, '?'),
    tipPopup,
  );

  function onIconFocus() {
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
      });
      tipPopup.classList.add('shown');
    });
  }

  tipContainer.onmouseover = onIconFocus;
  tipContainer.onfocus = onIconFocus;
  tipContainer.onmouseleave = resetPopupStyle;
  tipContainer.onblur = resetPopupStyle;

  return tipContainer;
}
