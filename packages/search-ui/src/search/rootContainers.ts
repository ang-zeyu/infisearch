import { computePosition, size, flip, arrow, Placement } from '@floating-ui/dom';
import { Searcher } from '@morsels/search-lib';

import { SearchUiOptions, UiOptions } from '../SearchUiOptions';
import { setCombobox, setInputAria } from '../utils/aria';
import h from '../utils/dom';
import createTipButton from './tips';


export function openDropdown(root: HTMLElement, listContainer: HTMLElement, placement: Placement) {
  if (listContainer.childElementCount) {
    const innerRoot = root.children[1] as HTMLElement;
    const caret = innerRoot.firstElementChild as HTMLElement;
    innerRoot.style.display = 'block';
    computePosition(root, innerRoot, {
      placement,
      middleware: [
        flip({
          padding: 10,
          mainAxis: false,
        }),
        size({
          apply({ availableWidth, availableHeight }) {
            Object.assign(listContainer.style, {
              maxWidth: `min(${availableWidth}px, var(--morsels-dropdown-max-width))`,
              maxHeight: `min(${availableHeight}px, var(--morsels-dropdown-max-height))`,
            });
          },
          padding: 10,
        }),
        arrow({
          element: caret,
        }),
      ],
    }).then(({ x, y, middlewareData }) => {
      Object.assign(innerRoot.style, {
        left: `${x}px`,
        top: `${y}px`,
      });
  
      const { x: arrowX } = middlewareData.arrow;
      Object.assign(caret.style, {
        left: arrowX != null ? `${arrowX}px` : '',
      });
    });
  }
}

export function closeDropdown(root: HTMLElement) {
  (root.children[1] as HTMLElement).style.display = 'none';
}

export function dropdownRootRender(
  uiOptions: UiOptions,
  searcher: Searcher,
  inputEl: HTMLInputElement,
) {
  const listContainer = h('ul', {
    id: 'morsels-dropdown-list',
    class: 'morsels-list',
  });
  const innerRoot = h('div',
    { class: 'morsels-inner-root', style: 'display: none;' },
    h('div', { class: 'morsels-input-dropdown-separator' }),
    listContainer,
  );
  createTipButton(innerRoot, uiOptions, searcher);
  const root = h('div', { class: 'morsels-root' },
    inputEl, innerRoot,
  );

  return [root, listContainer];
}

export function setDropdownInputAria(
  inputEl: HTMLElement, root: HTMLElement, listContainer: HTMLElement, label: string,
) {
  inputEl.removeAttribute('aria-label');
  setInputAria(inputEl, 'morsels-dropdown-list');
  setCombobox(root, listContainer, label);
}

export function unsetDropdownInputAria(
  combobox: HTMLElement,
  listbox: HTMLElement,
  input: HTMLElement,
  fsInputLabel: string,
) {
  combobox.removeAttribute('role');
  combobox.removeAttribute('aria-expanded');
  combobox.removeAttribute('aria-owns');
  listbox.removeAttribute('role');
  listbox.removeAttribute('aria-label');
  listbox.removeAttribute('aria-live');
  input.setAttribute('autocomplete', 'off');
  input.removeAttribute('aria-autocomplete');
  input.removeAttribute('aria-controls');
  input.removeAttribute('aria-activedescendant');
  input.setAttribute('aria-label', fsInputLabel);
}

export function openFullscreen(root: HTMLElement, listContainer: HTMLElement, fsContainer: HTMLElement) {
  fsContainer.appendChild(root);
  const input: HTMLInputElement = root.querySelector('input.morsels-fs-input');
  if (input) {
    input.focus();
  }

  const currentFocusedResult = listContainer.querySelector('.focus') as HTMLElement;
  if (currentFocusedResult) {
    listContainer.scrollTo({ top: currentFocusedResult.offsetTop - listContainer.offsetTop - 30 });
  }
}

export function closeFullscreen(root: HTMLElement) {
  root.remove();
}

export function fsRootRender(
  opts: SearchUiOptions,
  searcher: Searcher,
  fsCloseHandler: () => void,
) {
  const { uiOptions } = opts;
  const inputEl = h(
    'input', {
      class: 'morsels-fs-input',
      type: 'search',
      placeholder: uiOptions.fsPlaceholder,
      'aria-labelledby': 'morsels-fs-label',
    },
  ) as HTMLInputElement;
  setInputAria(inputEl, 'morsels-fs-list');

  const buttonEl = h('button', { class: 'morsels-input-close-fs' }, uiOptions.fsCloseText);
  buttonEl.onclick = (ev) => {
    ev.preventDefault();
    fsCloseHandler();
  };
  
  const listContainer = h('ul', {
    id: 'morsels-fs-list',
    class: 'morsels-list',
    'aria-labelledby': 'morsels-fs-label',
  });
    
  const innerRoot = h('div',
    { class: 'morsels-root morsels-fs-root' },
    h('form',
      { class: 'morsels-fs-input-button-wrapper' },
      h('label',
        { id: 'morsels-fs-label', for: 'morsels-fs-input', style: 'display: none' },
        uiOptions.label,
      ),
      inputEl,
      buttonEl,
    ),
    listContainer,
  );
  innerRoot.onclick = (ev) => ev.stopPropagation();
  innerRoot.onmousedown = (ev) => ev.stopPropagation();
  
  setCombobox(innerRoot, listContainer, uiOptions.label);
  createTipButton(innerRoot, uiOptions, searcher);
  
  const rootBackdropEl = h('div', { class: 'morsels-fs-backdrop' }, innerRoot);
  rootBackdropEl.onmousedown = fsCloseHandler;
  rootBackdropEl.onkeyup = (ev) => {
    if (ev.code === 'Escape') {
      ev.stopPropagation();
      fsCloseHandler();
    }
  };
  
  return [
    rootBackdropEl,
    listContainer,
    inputEl,
  ];
}
