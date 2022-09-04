import { computePosition, flip } from '@floating-ui/dom';
import { Searcher } from '@morsels/search-lib';
import { UiOptions } from '../Options';
import h from '../utils/dom';

export default function createTipButton(
  opts: UiOptions,
  searcher: Searcher,
): HTMLElement | string {
  if (opts.tip === false) {
    return;
  }

  function wrapInCode(example: string) {
    return h('code', {}, example);
  }

  function createRow(...contents: (string | HTMLElement)[]) {
    return h('tr', { class: 'morsels-tip-item' }, ...contents.map((el) => h('td', {}, h('div', {}, el))));
  }

  const tipListBody = h(
    'tbody', {},
    createRow(
      'Require all terms to match',
      wrapInCode('weather AND forecast AND sunny'),
    ),
    createRow(
      'Flip search results',
      wrapInCode('NOT rainy'),
    ),
    createRow(
      'Group terms together',
      wrapInCode('forecast AND (sunny warm)'),
    ),
    createRow(
      'Search for prefixes',
      wrapInCode('run*'),
    ),
    createRow(
      'Match specific areas',
      h('ul', {}, 
        h('li', {}, wrapInCode('title:forecast')),
        h('li', {}, wrapInCode('heading:sunny')),
        h('li', {}, wrapInCode('body:(rainy gloomy)')),
      ),
    ),
  );

  const tipList = h(
    'table',
    { class: 'morsels-tip-table' },
    h(
      'thead',
      { class: 'morsels-tip-table-header' },
      h('tr', {}, h('th', { scope: 'col' }, 'Tip'), h('th', {}, 'Example')),
    ),
    tipListBody,
  );
  const tipPopup = h(
    'div', { class: 'morsels-tip-popup-root' },
    h('div', { class: 'morsels-tip-popup' },
      h('div', { class: 'morsels-tip-popup-title' }, '🔎 Didn\'t find what you needed?'),
      tipList,
    ),
    h('div', { class: 'morsels-tip-popup-separator' }),
  );

  function resetPopupStyle() {
    Object.assign(tipPopup.style, {
      left: 'calc(var(--morsels-tip-icon-size) - 150px)',
      top: '-160px',
    });
    tipPopup.classList.remove('shown');
  }
  resetPopupStyle();

  const tipContainer = h(
    'div', { class: 'morsels-tip-root', tabindex: '0' },
    h('span', { class: 'morsels-tip-icon' }, '?'),
    tipPopup,
  );

  function onIconFocus() {
    computePosition(tipContainer, tipPopup, {
      placement: 'top-end',
      middleware: [
        flip({
          crossAxis: false,
          flipAlignment: false,
          padding: 10,
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

  searcher.setupPromise.then(() => {
    if (searcher.cfg.indexingConfig.withPositions) {
      tipListBody.prepend(createRow(
        'Search for phrases',
        wrapInCode('"for tomorrow"'),
      ));
    }
  });

  return tipContainer;
}
