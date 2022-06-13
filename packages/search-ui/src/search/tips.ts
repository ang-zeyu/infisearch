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

  function createListItem(content: string, example: string) {
    return h('li', { class: 'morsels-tip-item' },
      content,
      h('code', {}, example),
    );
  }

  const tipList = h(
    'ul',
    { class: 'morsels-tip-list' },
    createListItem(
      'Match multiple terms with "AND": ',
      'weather AND forecast AND sunny',
    ),
    createListItem('Flip results with "NOT": ', 'NOT rainy'),
    createListItem(
      'Match 1 of 3 specific parts of pages: ',
      'title:forecast or heading:sunny or body:rainy',
    ),
    createListItem('Group terms or expressions into a expression with brackets: ', '(...expressions...)'),
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
      tipList.append(createListItem('Search for phrases using quotes: ', '"for tomorrow"'));
    }
  });
}
