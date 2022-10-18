import { computePosition, size, flip, arrow, Placement } from '@floating-ui/dom';
import { Searcher } from '@morsels/search-lib';

import { Options, UiOptions } from '../Options';
import { setInputAria, unsetActiveDescendant, unsetExpanded } from '../utils/aria';
import h from '../utils/dom';
import createTipButton from './tips';


export function openDropdown(root: HTMLElement, listContainer: HTMLElement, placement: Placement) {
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

export function closeDropdown(root: HTMLElement) {
  (root.children[1] as HTMLElement).style.display = 'none';
}

// Incremental Id for pages with multiple UIs, for aria attributes.
let dropdownId = 0;

export function dropdownRootRender(
  uiOptions: UiOptions,
  searcher: Searcher,
  inputEl: HTMLInputElement,
  hideDropdown: () => void,
) {
  const listContainer = h('div', {
    id: `morsels-dropdown-list-${dropdownId++}`,
    class: 'morsels-list',
    // Prevent dropdown from being dismissed when clicking anywhere else inside
    tabindex: '-1',
  });
  const innerRoot = h('div',
    { class: 'morsels-inner-root', style: 'display: none;' },
    h('div', { class: 'morsels-input-dropdown-separator' }),
    createTipButton(uiOptions, searcher),
    listContainer,
  );
  
  const root = h('div', { class: 'morsels-root morsels-dropdown-root' },
    inputEl, innerRoot,
  );
  innerRoot.onkeyup = (ev) => {
    if (ev.code === 'Escape') {
      ev.stopPropagation();
      inputEl.focus();
      hideDropdown();
    }
  };

  return [root, listContainer];
}

export function setFsTriggerInput(input: HTMLElement, fsInputButtonText: string, fsInputLabel: string) {
  input.setAttribute('autocomplete', 'off');
  input.setAttribute('readonly', '');
  input.setAttribute('role', 'button');
  input.setAttribute('aria-label', fsInputLabel);
  if (fsInputButtonText) input.setAttribute('placeholder', fsInputButtonText);
  input.classList.add('morsels-button-input');
}

function unsetFsTriggerInput(input: HTMLElement, originalPlaceholder: string) {
  input.removeAttribute('readonly');
  input.removeAttribute('role');
  input.removeAttribute('aria-label');
  input.setAttribute('placeholder', originalPlaceholder);
  input.classList.remove('morsels-button-input');
}

export function setDropdownInputAria(
  input: HTMLElement,
  listContainer: HTMLElement,
  label: string,
  originalPlaceholder: string,
) {
  unsetFsTriggerInput(input, originalPlaceholder);
  setInputAria(input, listContainer, label);
}

export function unsetDropdownInputAria(
  input: HTMLElement,
  listbox: HTMLElement,
  fsInputLabel: string,
  fsInputButtonText: string,
) {
  listbox.removeAttribute('role');
  listbox.removeAttribute('aria-label');
  listbox.removeAttribute('aria-live');
  input.removeAttribute('role');
  unsetExpanded(input);
  input.removeAttribute('aria-autocomplete');
  input.removeAttribute('aria-controls');
  unsetActiveDescendant(input);
  setFsTriggerInput(input, fsInputButtonText, fsInputLabel);
}

// Incremental Id for pages with multiple UIs, for aria attributes.
let fsId = 0;

export function fsRootRender(
  opts: Options,
  searcher: Searcher,
  onClose: (isKeyboardClose: boolean) => void,
): [HTMLElement, HTMLElement, HTMLInputElement, () => void, (isKeyboardClose: boolean) => void] {
  const { uiOptions } = opts;

  const labelId = `morsels-fs-label-${fsId}`;
  const inputEl = h(
    'input', {
      class: 'morsels-fs-input',
      type: 'search',
      placeholder: uiOptions.fsPlaceholder,
      'aria-labelledby': labelId,
      'enterkeyhint': 'search',
    },
  ) as HTMLInputElement;
  inputEl.onkeydown = (ev) => {
    if (ev.key === 'Escape' && inputEl.value) {
      ev.stopPropagation();
    }
  };

  const inputClearEl = h('span', { class: 'morsels-fs-input-clear' });
  inputClearEl.onclick = () => {
    if (inputEl.value) {
      inputEl.value = '';
      inputEl.dispatchEvent(new KeyboardEvent('input'));
      inputEl.focus();
    }
  };

  const buttonEl = h('button', { class: 'morsels-input-close-fs' }, uiOptions.fsCloseText);
  
  const listContainer = h('div', {
    id: `morsels-fs-list-${fsId++}`,
    class: 'morsels-list',
    'aria-labelledby': labelId,
  });

  const innerRoot = h('div',
    { class: 'morsels-root morsels-fs-root' },
    h('form',
      { class: 'morsels-fs-controls' },
      h('label',
        { id: labelId, for: 'morsels-fs-input', style: 'display: none' },
        uiOptions.label,
      ),
      h('div',
        { class: 'morsels-fs-input-wrapper' },
        inputEl, inputClearEl,
      ),
      buttonEl,
    ),
    createTipButton(uiOptions, searcher),
    listContainer,
  );
  innerRoot.onclick = (ev) => ev.stopPropagation();
  innerRoot.onmousedown = (ev) => ev.stopPropagation();
  
  setInputAria(inputEl, listContainer, uiOptions.label);
  
  const rootBackdropEl = h('div', { class: 'morsels-fs-backdrop' }, innerRoot);

  function hideFullscreen(isKeyboardClose: boolean) {
    onClose(isKeyboardClose);
    rootBackdropEl.remove();
  }

  rootBackdropEl.onmousedown = () => hideFullscreen(false);
  rootBackdropEl.onkeydown = (ev) => {
    if (ev.code === 'Escape') {
      ev.stopPropagation();
      hideFullscreen(true);
    }
  };

  buttonEl.onclick = (ev: PointerEvent) => {
    ev.preventDefault();
    hideFullscreen(ev.pointerType === '');
  };

  function openFullscreen() {
    uiOptions.fsContainer.appendChild(rootBackdropEl);
    const input: HTMLInputElement = rootBackdropEl.querySelector('input.morsels-fs-input');
    if (input) {
      input.focus();
    }
  
    const currentFocusedResult = listContainer.querySelector('.focus') as HTMLElement;
    if (currentFocusedResult) {
      listContainer.scrollTo({ top: currentFocusedResult.offsetTop - listContainer.offsetTop - 30 });
    }
  }
  
  return [
    rootBackdropEl,
    listContainer,
    inputEl,
    openFullscreen,
    hideFullscreen,
  ];
}
