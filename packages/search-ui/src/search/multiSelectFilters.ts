import h from '@morsels/search-lib/lib/utils/dom';

import { MorselsConfig } from '@morsels/search-lib/lib/results/Config';
import { Options } from '../Options';
import { IManager } from '../InputManager';
import { unsetActiveDescendant } from '../utils/aria';

const OPTION_ENTER_EV = 'morsels-filter-opt-enter';

let tieBreaker = 0;

export interface MultiSelectState {
  readonly _mrlIdx: number,
  readonly _mrlFieldName: string,
  readonly _mrlDisplayName: string,
  readonly _mrlEnumNames: string[],
  readonly _mrlIsEnumActive: boolean[],
}

function getMultiSelectStates(
  opts: Options, cfg: MorselsConfig,
): MultiSelectState[] {
  const multiSelectBindings = opts.uiOptions.multiSelectFilters;
  const fieldInfos = cfg.fieldInfos;

  return multiSelectBindings
    .filter(({ fieldName }) => fieldInfos.find(({ name }) => fieldName === name))
    .map(({ fieldName, displayName, defaultOptName }, idx) => {
      const fieldInfo = fieldInfos.find(({ name }) => fieldName === name);
      const enumValues = [defaultOptName, ...fieldInfo.enumInfo.enumValues];
      const state = {
        _mrlIdx: idx,
        _mrlFieldName: fieldName,
        _mrlDisplayName: displayName,
        _mrlEnumNames: enumValues,
        _mrlIsEnumActive: enumValues.map(() => true),
      };

      return state;
    });
}

function renderFilterHeader(iManager: IManager, state: MultiSelectState) {
  const headerIdTieBreaker = tieBreaker++;
  const id = 'morsels-filter-opts-' + headerIdTieBreaker;

  const filterOptions = h('div', {
    id,
    role: 'listbox',
    'aria-multiselectable': 'true',
    'aria-label': 'filter options',
  });

  const filterHeader = h('div', {
    class: 'morsels-filter-header',
    tabindex: '0',
    role: 'combobox',
    'aria-expanded': 'false',
    'aria-label': 'filter',
  }, state._mrlDisplayName);

  const container = h('div',
    { class: 'morsels-filter' },
    filterHeader,
    filterOptions,
  );

  function visualFocusEl(el: Element, focusedEl: HTMLElement) {
    if (focusedEl) {
      focusedEl.classList.remove('focus');
    }
  
    if (el) {
      el.classList.add('focus');
      filterHeader.setAttribute('aria-activedescendant', el.getAttribute('id'));
    } else {
      unsetActiveDescendant(filterHeader);
    }
  }

  function getFocusedItem(): [number, HTMLElement, NodeListOf<Element>] {
    const allItems = filterOptions.querySelectorAll('[role="option"]');
    let focusedItem: HTMLElement;
    let focusedItemIdx = -1;
    allItems.forEach((el, idx) => {
      if ((el as Element).getAttribute('id') === filterHeader.getAttribute('aria-activedescendant')) {
        focusedItem = el as HTMLElement;
        focusedItemIdx = idx;
      }
    });
    return [focusedItemIdx, focusedItem, allItems];
  }

  function renderFilterOption(
    enumName: string,
    idx: number,
  ) {
    const input = h('input', {
      type: 'checkbox',
      class: 'morsels-checkbox',
      checked: 'true',
      role: 'option',
      'aria-selected': 'true',
      id: `morsels-filter-opt-${headerIdTieBreaker}-${idx}`,
    }) as HTMLInputElement;
  
    const opt = h('div',
      { class: 'morsels-filter-opt' },
      h('label', { class: 'morsels-checkbox-label' }, input, enumName),
    );

    function focusOption(addVisualFocus: boolean) {
      state._mrlIsEnumActive[idx] = input.checked;
      input.setAttribute('aria-selected', input.checked + '');

      const [, focusedItem] = getFocusedItem();
      if (addVisualFocus) {
        visualFocusEl(input, focusedItem);
      }
  
      if (iManager._mrlInputEl.value)
        iManager._mrlQueueNewQuery(iManager._mrlInputEl.value);
    }
  
    // Spacebar or Raw click
    input.onclick = (ev) => {
      ev.stopPropagation();
      focusOption(false);
    };

    // Combobox
    input.addEventListener(OPTION_ENTER_EV, () => {
      input.checked = !input.checked;
      focusOption(true);
    });
  
    return opt;
  }

  const childOpts = state._mrlEnumNames.map((name, optIdx) => renderFilterOption(
    name, optIdx,
  ));

  function expandHeader() {
    filterOptions.innerHTML = '';
    filterOptions.append(...childOpts);

    filterHeader.classList.add('active');
    filterHeader.setAttribute('aria-expanded', 'true');
    filterHeader.setAttribute('aria-controls', id);
  }

  function collapseHeader() {
    const [, focusedItem] = getFocusedItem();
    visualFocusEl(undefined, focusedItem);

    filterOptions.innerHTML = '';

    filterHeader.classList.remove('active');
    filterHeader.setAttribute('aria-expanded', 'false');
    filterHeader.removeAttribute('aria-controls');
    unsetActiveDescendant(filterHeader);
  }

  // Expand the first header
  let shown = state._mrlIdx === 0;
  const showOrHideOptions = () => {
    if (shown) {
      collapseHeader();
    } else {
      expandHeader();
    }

    shown = !shown;
  };

  if (shown) {
    expandHeader();
  }

  filterHeader.onclick = showOrHideOptions;
  filterHeader.onkeydown = (ev) => {
    if (!['ArrowDown', 'ArrowUp', 'Enter', ' ', 'Home', 'End', 'Escape'].includes(ev.key)) {
      return;
    }

    const key = ev.key;

    ev.preventDefault(); // prevent scroll

    const [focusedItemIdx, focusedItem, allItems] = getFocusedItem();
    const firstItem = allItems[0] as HTMLElement;
    const lastItem = allItems[allItems.length - 1] as HTMLElement;

    if (shown) {
      if (key === 'ArrowDown') {
        visualFocusEl(
          allItems[(focusedItemIdx + 1) % allItems.length] as Element, focusedItem,
        );
      } else if (key === 'ArrowUp') {
        visualFocusEl(
          (focusedItemIdx > 0 ? allItems[focusedItemIdx - 1] : lastItem) as Element, focusedItem,
        );
      } else if (key === 'Enter' || key === ' ') {
        if (focusedItem) {
          focusedItem.dispatchEvent(new Event(OPTION_ENTER_EV));
          ev.stopPropagation();
        } else if (key === 'Enter') {
          showOrHideOptions();
        }
      } else if (key === 'Home') {
        visualFocusEl(firstItem, focusedItem);
      } else if (key === 'End') {
        visualFocusEl(lastItem, focusedItem);
      } else if (key === 'Escape') {
        showOrHideOptions();
        ev.stopPropagation();
      }
    } else if (key === 'Enter') {
      showOrHideOptions();
    }
  };

  return container;
}

export function filtersRender(
  opts: Options,
  cfg: MorselsConfig,
  iManager: IManager,
): [HTMLElement, MultiSelectState[], (setValue?: boolean) => boolean] {
  const states = getMultiSelectStates(opts, cfg);
  
  const filters = h('div', {},
    ...states.map((state) => renderFilterHeader(iManager, state)),
  );

  const filtersContainer = h('div', { class: 'morsels-filters' });

  let shown = false;
  const getOrSetFiltersShown = states.length ? (setValue?: boolean) => {
    if (setValue === undefined || shown === setValue) {
      return shown;
    }

    if (shown) {
      filters.remove();
      filtersContainer.classList.remove('shown');
    } else {
      filtersContainer.prepend(filters);
      filtersContainer.classList.add('shown');
    }

    return shown = setValue;
  } : undefined;

  return [filtersContainer, states, getOrSetFiltersShown];
}
