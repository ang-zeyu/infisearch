import { computePosition, flip } from '@floating-ui/dom';
import { Searcher } from '@morsels/search-lib';
import { UiOptions } from '../SearchUiOptions';
import h from '../utils/dom';

export default function createTipButton(
  root: HTMLElement,
  opts: UiOptions,
  searcher: Searcher,
): HTMLElement | string {
  if (opts.tip === false) {
    return;
  }

  function wrapInCode(example: string) {
    return h('code', {}, example);
  }

  function createListItem(...contents: (string | HTMLElement)[]) {
    return h('li', { class: 'morsels-tip-item' }, ...contents);
  }

  const tipList = h(
    'ul',
    { class: 'morsels-tip-list' },
    createListItem(
      'Match multiple terms or expressions:',
      wrapInCode('weather AND forecast AND sunny'),
    ),
    createListItem(
      'Flip results for any expression:',
      wrapInCode('NOT rainy'),
    ),
    createListItem(
      'Match 1 of 3 specific areas of pages:',
      h('ul', {}, 
        h('li', {}, wrapInCode('title:forecast')),
        h('li', {}, wrapInCode( 'heading:sunny')),
        h('li', {}, wrapInCode('body:rainy')),
      ),
    ),
    createListItem(
      'Group/nest expressions together:',
      wrapInCode('forecast AND (sunny warm)'),
    ),
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

  root.append(tipContainer);

  searcher.setupPromise.then(() => {
    if (searcher.cfg.indexingConfig.withPositions) {
      tipList.append(createListItem(
        'Search for phrases using quotes: ',
        wrapInCode('"for tomorrow"'),
      ));
    }
  });
}
