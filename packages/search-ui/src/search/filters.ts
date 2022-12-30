import h from '@infisearch/search-lib/lib/utils/dom';

import { InfiConfig } from '@infisearch/search-lib/lib/results/Config';
import { NumericFilterBinding, Options, UiOptions } from '../Options';
import { IManager } from '../InputManager';
import { unsetActiveDescendant } from '../utils/aria';

const OPTION_ENTER_EV = 'infi-multi-opt-enter';

let tieBreaker = 0;

export interface MultiSelectState {
  readonly _mrlIdx: number,
  readonly _mrlFieldName: string,
  readonly _mrlDisplayName: string,
  readonly _mrlEnumNames: string[],
  readonly _mrlIsEnumActive: boolean[],
  readonly _mrlInitialExpanded: boolean,
}

export interface NumericFilterState {
  readonly _mrlBinding: NumericFilterBinding,
  _mrlGte?: number | bigint,
  _mrlLte?: number | bigint,
}

export interface FilterSortStates {
  _mrlMultiSelects: MultiSelectState[],
  _mrlNumericFilters: NumericFilterState[],
  _mrlSortChoice: string | null,  // null = default (relevance)
  _mrlSortAscending: boolean,     // default = descending
}

function getFilterSortStates(
  opts: Options, cfg: InfiConfig,
): FilterSortStates {
  const { multiSelectFilters, numericFilters } = opts.uiOptions;
  const fieldInfos = cfg.fieldInfos;

  return {
    _mrlMultiSelects: multiSelectFilters
      .filter(({ fieldName }) => fieldInfos.find(({ name }) => fieldName === name))
      .map(({ fieldName, displayName, defaultOptName, collapsed }, idx) => {
        const fieldInfo = fieldInfos.find(({ name }) => fieldName === name);
        const enumValues = [defaultOptName, ...fieldInfo.enumInfo.enumValues];
        const state = {
          _mrlIdx: idx,
          _mrlFieldName: fieldName,
          _mrlDisplayName: displayName,
          _mrlEnumNames: enumValues,
          _mrlIsEnumActive: enumValues.map(() => true),
          // Expand the first header
          _mrlInitialExpanded: collapsed === undefined ? (idx === 0) : collapsed,
        };

        return state;
      }),
    _mrlNumericFilters: numericFilters
      .filter(({ fieldName }) => fieldInfos.find(({ name }) => fieldName === name))
      .map((binding) => ({
        _mrlBinding: binding,
        _mrlGte: binding.gte,
        _mrlLte: binding.lte,
      })),
    _mrlSortChoice: null,
    _mrlSortAscending: false,
  };
}

function renderMultiSelectFilter(iManager: IManager, state: MultiSelectState) {
  const headerIdTieBreaker = tieBreaker++;
  const id = 'infi-multi-opts-' + headerIdTieBreaker;

  const filterOptions = h('div', {
    id,
    role: 'listbox',
    'aria-multiselectable': 'true',
    'aria-label': 'filter options',
  });

  const filterHeader = h('div', {
    class: 'infi-multi-header',
    tabindex: '0',
    role: 'combobox',
    'aria-expanded': 'false',
    'aria-label': 'filter',
  }, state._mrlDisplayName);

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
      class: 'infi-checkbox',
      checked: 'true',
      role: 'option',
      'aria-selected': 'true',
      id: `infi-multi-opt-${headerIdTieBreaker}-${idx}`,
    }) as HTMLInputElement;
  
    const opt = h('div',
      { class: 'infi-multi' },
      h('label', { class: 'infi-checkbox-label' }, input, enumName),
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

  let shown = state._mrlInitialExpanded;
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

  return h('div', {}, filterHeader, filterOptions);
}

function renderNumericFilter(iManager: IManager, state: NumericFilterState) {
  const { type, displayName, minLabel, maxLabel } = state._mrlBinding;

  const onChange = (isGte: boolean) => (ev: Event) => {
    const rawValue = (ev.target as HTMLInputElement).value;
    let value: number | bigint;
    try {
      const intPart = rawValue.split('.')[0];
      value = type === 'number'

        // Round down
        // BigInt('') = 0n, so, set it to undefined
        ? (intPart.trim() ? BigInt(intPart) : undefined)

        // Date constructor will not throw errors (ends up as NaN), but BigInt will.
        : (BigInt(+new Date(rawValue)) / BigInt(1000));
    } catch (ex) {
      // Parsing error, set value = undefined
    }

    if (isGte) {
      state._mrlGte = value;
    } else {
      state._mrlLte = value;
    }

    const query = iManager._mrlInputEl.value;
    if (query) iManager._mrlQueueNewQuery(query);
  };

  const isDateTime = type.startsWith('date');
  const minText = minLabel || (isDateTime ? 'After' : 'Min');
  const maxText = maxLabel || (isDateTime ? 'Before' : 'Max');

  function srOnlyText(text: string) {
    return h('span', { class: 'infi-sr-only' }, text);
  }

  function getInput(label: string, isMin: boolean): HTMLElement {
    const input = h('input', {
      class: 'infi-minmax',
      placeholder: label,
      type,
    });
    input.onchange = onChange(isMin);

    return h('label', {},
      srOnlyText(displayName),
      isDateTime ? h('span', { class: 'infi-minmax-label' }, label) : srOnlyText(label),
      input,
    );
  }

  const el = h(
    'div',
    { class: 'infi-min-max' },
    h('div', { class: 'infi-filter-header' }, displayName),
    getInput(minText, true),
    ' - ',
    getInput(maxText, false),
  );

  return el;
}

function sortOptionRender(fieldName: string, isAscending: number, label: string): HTMLElement {
  return h('option', { value: `${fieldName}<->${isAscending}` }, label);
}

function sortFieldsRender(iManager: IManager, opts: UiOptions, states: FilterSortStates) {
  const { sortFields, translations } = opts;
  const sortFieldsEntries = Object.entries(sortFields);
  if (!sortFieldsEntries.length) {
    return '';
  }

  const sortOptionEls = [
    h('option', { value: 'relevance', selected: 'true' }, 'Relevance'),
  ];
  sortFieldsEntries.forEach(([fieldName, { asc, desc }]) => {
    if (asc) {
      sortOptionEls.push(sortOptionRender(fieldName, 1, asc));
    }

    if (desc) {
      sortOptionEls.push(sortOptionRender(fieldName, 0, desc));
    }
  });

  const id = `infi-sort-${tieBreaker++}`;
  const selectEl = h('select', {
    class: 'infi-sort',
    id,
  }, ...sortOptionEls);
  selectEl.onchange = (ev: any) => {
    const [fieldName, isAscending] = ev.target.value.split('<->');
    states._mrlSortChoice = fieldName;
    states._mrlSortAscending = !!Number(isAscending);

    const query = iManager._mrlInputEl.value;
    if (query) iManager._mrlQueueNewQuery(query);
  };

  return h('div', {},
    h('label', { class: 'infi-filter-header', for: id }, translations.sortBy),
    selectEl,
  );
}

export function filtersRender(
  opts: Options,
  cfg: InfiConfig,
  iManager: IManager,
): [HTMLElement, FilterSortStates, (setValue?: boolean) => boolean] {
  const states = getFilterSortStates(opts, cfg);
  
  const sortFields = sortFieldsRender(iManager, opts.uiOptions, states);
  const numericFilters = states._mrlNumericFilters.map((state) => renderNumericFilter(iManager, state));
  const multiSelectFilters = states._mrlMultiSelects.map((state) => renderMultiSelectFilter(iManager, state));

  const hasAnyControls = sortFields || numericFilters.length || multiSelectFilters.length;

  const filters = h('div', {},
    sortFields,
    (sortFields && (numericFilters.length || multiSelectFilters.length))
      ? h('hr', { class: 'infi-sep' })
      : '',
    ...numericFilters,
    (numericFilters.length && multiSelectFilters.length)
      ? h('hr', { class: 'infi-sep' })
      : '',
    ...multiSelectFilters,
    hasAnyControls ? h('hr', { class: 'infi-sep' }) : '',
  );

  const filtersContainer = h('div', { class: 'infi-filters' });

  let shown = false;
  const getOrSetFiltersShown = hasAnyControls ? (setValue?: boolean) => {
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
