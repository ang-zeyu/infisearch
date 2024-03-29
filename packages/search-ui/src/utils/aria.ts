import { SELECTED_OPTION_ID } from './keyboard';

export function setActiveDescendant(input: HTMLElement) {
  input.setAttribute('aria-activedescendant', SELECTED_OPTION_ID);
}

export function setExpanded(combobox: HTMLElement) {
  combobox.setAttribute('aria-expanded', 'true');
}

export function unsetActiveDescendant(input: HTMLElement) {
  input.removeAttribute('aria-activedescendant');
}

export function unsetExpanded(combobox: HTMLElement) {
  combobox.setAttribute('aria-expanded', 'false');
}

export function setInputAria(input: HTMLElement, listbox: HTMLElement, listboxLabel: string) {
  input.setAttribute('role', 'combobox');
  input.setAttribute('autocomplete', 'off');
  input.setAttribute('aria-autocomplete', 'list');
  const listId = listbox.getAttribute('id');
  input.setAttribute('aria-controls', listId);
  unsetExpanded(input);
  listbox.setAttribute('role', 'listbox');
  listbox.setAttribute('aria-label', listboxLabel);
}
