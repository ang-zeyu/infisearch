import { setActiveDescendant, unsetActiveDescendant } from './aria';
import { scrollListContainer } from './scrollListContainer';

export const SELECTED_OPTION_ID = 'infi-list-selected';

export function focusEl(
  el: Element,
  focusedEl: HTMLElement,
  inputEl: HTMLInputElement,
  scrollContainer: HTMLElement,
  doScroll: boolean,
) {
  if (focusedEl) {
    focusedEl.classList.remove('focus');
    focusedEl.removeAttribute('aria-selected');
    focusedEl.removeAttribute('id');
  }

  if (el) {
    el.classList.add('focus');
    el.setAttribute('aria-selected', 'true');
    el.setAttribute('id', SELECTED_OPTION_ID);
    if (doScroll) scrollListContainer(el, scrollContainer);
    setActiveDescendant(inputEl);
  } else {
    if (doScroll) scrollContainer.scrollTo({ top: 0 });
    unsetActiveDescendant(inputEl);
  }
}

export function addKeyboardHandler(
  inputEl: HTMLInputElement,
  resultContainer: HTMLElement,
  scrollContainer: HTMLElement,
) {
  inputEl.addEventListener('keydown', (ev: KeyboardEvent) => {
    const { key } = ev;
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter'].includes(key)) {
      return;
    }

    const focusedItem = resultContainer.querySelector(`#${SELECTED_OPTION_ID}`) as HTMLElement;

    const opts = resultContainer.querySelectorAll('[role="option"]');
    const lastItem = opts[opts.length - 1];

    let focusedItemIdx = -1;
    opts.forEach((v, idx) => {
      if (v === focusedItem) {
        focusedItemIdx = idx;
      }
    });

    if (key === 'ArrowDown') {
      focusEl(opts[(focusedItemIdx + 1) % opts.length], focusedItem, inputEl, scrollContainer, true);
    } else if (key === 'ArrowUp') {
      focusEl(
        focusedItemIdx > 0 ? opts[focusedItemIdx - 1] : lastItem, focusedItem, inputEl, scrollContainer, true,
      );
    } else if (key === 'Enter') {
      if (focusedItem)
        focusedItem.dispatchEvent(new MouseEvent('click', {
          ctrlKey: ev.ctrlKey,
          cancelable: true,
        }));
    } else {
      const pos = key === 'Home' ? 0 : inputEl.value.length;
      inputEl.focus();
      inputEl.setSelectionRange(pos, pos);
      focusEl(undefined, focusedItem, inputEl, scrollContainer, true);
    }

    ev.preventDefault();
  });
}