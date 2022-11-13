import { computePosition, size, flip, arrow, Placement } from '@floating-ui/dom';
import h from '@infisearch/search-lib/lib/utils/dom';

import { Options } from '../Options';
import { setInputAria, unsetActiveDescendant } from '../utils/aria';


export function openDropdown(
  input: HTMLInputElement,
  root: HTMLElement,
  listContainer: HTMLElement,
  placement: Placement,
) {
  const innerRoot = root.children[1] as HTMLElement;
  const caret = innerRoot.firstElementChild as HTMLElement;
  innerRoot.style.display = 'block';
  computePosition(input, innerRoot, {
    placement,
    middleware: [
      flip({
        padding: 10,
        mainAxis: false,
      }),
      size({
        apply({ availableWidth, availableHeight }) {
          Object.assign(listContainer.style, {
            maxWidth: `min(${availableWidth}px, var(--infi-dropdown-max-width))`,
            maxHeight: `min(${availableHeight}px, var(--infi-dropdown-max-height))`,
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

function getTemporaryElements() {
  // Placeholders, swapped in later in InputManager
  return [
    h('div', {}),
    h('div', { class: 'infi-filters' }),
    h('div', {}),
  ];
}

// Incremental Id for pages with multiple UIs, for aria attributes.
let resultContainerId = 0;

export function dropdownRootRender(
  opts: Options,
  inputEl: HTMLInputElement,
  hideDropdown: () => void,
) {
  const resultContainer = h('div', {
    id: `infi-dropdown-list-${resultContainerId++}`,
  });
  const scrollContainer = h('div',
    {
      class: 'infi-list',
      // Prevent dropdown from being dismissed when clicking anywhere else inside
      tabindex: '-1',
    },
    ...getTemporaryElements(),
    resultContainer,
  );
  const innerRoot = h('div',
    { class: 'infi-inner-root', style: 'display: none;' },
    h('div', { class: 'infi-input-dropdown-separator' }),
    scrollContainer,
  );
  
  const root = h('div', { class: 'infi-root infi-dropdown-root' },
    inputEl, innerRoot,
  );
  innerRoot.onkeydown = (ev) => {
    if (ev.code === 'Escape') {
      ev.stopPropagation();
      inputEl.focus();
      hideDropdown();
    }
  };

  return [root, scrollContainer];
}

export function setFsTriggerInput(input: HTMLElement, fsInputButtonText: string, fsInputLabel: string) {
  input.setAttribute('autocomplete', 'off');
  input.setAttribute('readonly', '');
  input.setAttribute('role', 'button');
  input.setAttribute('aria-label', fsInputLabel);
  if (fsInputButtonText) input.setAttribute('placeholder', fsInputButtonText);
  input.classList.add('infi-button-input');
}

function unsetFsTriggerInput(input: HTMLElement, originalPlaceholder: string) {
  input.removeAttribute('readonly');
  input.removeAttribute('role');
  input.removeAttribute('aria-label');
  input.setAttribute('placeholder', originalPlaceholder);
  input.classList.remove('infi-button-input');
}

export function setDropdownInputAria(
  input: HTMLElement,
  resultContainer: HTMLElement,
  label: string,
  originalPlaceholder: string,
) {
  unsetFsTriggerInput(input, originalPlaceholder);
  setInputAria(input, resultContainer, label);
}

export function unsetDropdownInputAria(
  input: HTMLElement,
  resultContainer: HTMLElement,
  fsInputLabel: string,
  fsInputButtonText: string,
) {
  resultContainer.removeAttribute('role');
  resultContainer.removeAttribute('aria-label');
  input.removeAttribute('role');
  input.removeAttribute('aria-expanded');
  input.removeAttribute('aria-autocomplete');
  input.removeAttribute('aria-controls');
  unsetActiveDescendant(input);
  setFsTriggerInput(input, fsInputButtonText, fsInputLabel);
}

// Incremental Id for pages with multiple UIs, for aria attributes.
let fsId = 0;

export function fsRootRender(
  opts: Options,
  onClose: (isKeyboardClose: boolean) => void,
): [HTMLElement, HTMLInputElement, () => void, (isKeyboardClose: boolean) => void] {
  const {
    fsPlaceholder,
    fsCloseText,
    fsContainer,
    label,
  } = opts.uiOptions;

  const labelId = `infi-fs-label-${fsId}`;
  const inputEl = h(
    'input', {
      class: 'infi-fs-input',
      type: 'search',
      placeholder: fsPlaceholder,
      'aria-labelledby': labelId,
      'enterkeyhint': 'search',
    },
  ) as HTMLInputElement;
  inputEl.onkeydown = (ev) => {
    if (ev.key === 'Escape' && inputEl.value) {
      ev.stopPropagation();
    }
  };

  const inputClearEl = h('span', { class: 'infi-fs-input-clear' });
  inputClearEl.onclick = () => {
    if (inputEl.value) {
      inputEl.value = '';
      inputEl.dispatchEvent(new KeyboardEvent('input'));
      inputEl.focus();
    }
  };

  const buttonEl = h('button', {
    class: 'infi-input-close-fs',
    type: 'button',
  }, fsCloseText);
  
  const resultContainer = h('div', {
    id: `infi-fs-list-${fsId++}`,
    'aria-labelledby': labelId,
  });
  const scrollContainer = h('div',
    { class: 'infi-list', tabindex: '-1' },
    ...getTemporaryElements(),
    resultContainer,
  );

  const innerRoot = h('div',
    { class: 'infi-root infi-fs-root' },
    h('div',
      { class: 'infi-fs-controls' },
      h('div',
        { class: 'infi-fs-input-wrapper' },
        inputEl, inputClearEl,
      ),
      buttonEl,
    ),
    scrollContainer,
  );
  innerRoot.onclick = (ev) => ev.stopPropagation();
  innerRoot.onmousedown = (ev) => ev.stopPropagation();
  
  setInputAria(inputEl, resultContainer, label);
  
  const rootBackdropEl = h('div', { class: 'infi-fs-backdrop' }, innerRoot);

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
    fsContainer.appendChild(rootBackdropEl);
    inputEl.focus();
  
    const currentFocusedResult = resultContainer.querySelector('.focus') as HTMLElement;
    if (currentFocusedResult) {
      scrollContainer.scrollTo({ top: currentFocusedResult.offsetTop - scrollContainer.offsetTop - 30 });
    }
  }
  
  return [
    scrollContainer,
    inputEl,
    openFullscreen,
    hideFullscreen,
  ];
}

export function targetRender(
  opts: Options,
  input: HTMLInputElement,
  target: HTMLElement,
) {
  target.classList.add('infi-root');

  const resultContainer = h('div', { id: `infi-target-list-${resultContainerId++}` });
  target.append(
    ...getTemporaryElements(),
    resultContainer,
  );

  setInputAria(input, resultContainer, opts.uiOptions.label);
}
