import { setActiveDescendant, unsetActiveDescendant } from './aria';

export const SELECTED_OPTION_ID = 'morsels-list-selected';

function scrollListContainer(targetEl: any, listContainer: HTMLElement) {
  const computedStyles = getComputedStyle(listContainer);
  if (['scroll', 'auto', 'overlay'].includes(computedStyles.overflowY)) {
    const top = targetEl.offsetTop
      - listContainer.offsetTop
      - listContainer.clientHeight / 2
      + targetEl.clientHeight / 2;
    listContainer.scrollTo({ top });
  } else {
    targetEl.scrollIntoView({
      block: 'center',
    });
  }
}

export function focusEl(
  el: Element,
  focusedEl: HTMLElement,
  inputEl: HTMLInputElement,
  listContainer: HTMLElement,
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
    if (doScroll) scrollListContainer(el, listContainer);
    setActiveDescendant(inputEl);
  } else {
    if (doScroll) listContainer.scrollTo({ top: 0 });
    unsetActiveDescendant(inputEl);
  }
}

export function addKeyboardHandler(inputEl: HTMLInputElement, listContainer: HTMLElement) {
  inputEl.addEventListener('keydown', (ev: KeyboardEvent) => {
    const { key } = ev;
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter'].includes(key)) {
      return;
    }

    const focusedItem = listContainer.querySelector('#morsels-list-selected') as HTMLElement;

    const opts = listContainer.querySelectorAll('[role="option"]');
    const lastItem = opts[opts.length - 1];

    let focusedItemIdx = -1;
    opts.forEach((v, idx) => {
      if (v === focusedItem) {
        focusedItemIdx = idx;
      }
    });

    if (key === 'ArrowDown') {
      focusEl(opts[(focusedItemIdx + 1) % opts.length], focusedItem, inputEl, listContainer, true);
    } else if (key === 'ArrowUp') {
      focusEl(
        focusedItemIdx > 0 ? opts[focusedItemIdx - 1] : lastItem, focusedItem, inputEl, listContainer, true,
      );
    } else if (key === 'Enter') {
      if (focusedItem)
        focusedItem.dispatchEvent(new MouseEvent('click', {
          ctrlKey: ev.ctrlKey,
        }));
    } else {
      const pos = key === 'Home' ? 0 : inputEl.value.length;
      inputEl.focus();
      inputEl.setSelectionRange(pos, pos);
      focusEl(undefined, focusedItem, inputEl, listContainer, true);
    }

    ev.preventDefault();
  });
}