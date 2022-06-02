export function setCombobox(combobox: HTMLElement, listbox: HTMLElement, label: string) {
  combobox.setAttribute('role', 'combobox');
  combobox.setAttribute('aria-expanded', 'true');
  combobox.setAttribute('aria-owns', listbox.getAttribute('id'));
  listbox.setAttribute('role', 'listbox');
  listbox.setAttribute('aria-label', label);
  listbox.setAttribute('aria-live', 'polite');
}
  
export function setInputAria(input: HTMLElement, listId: string) {
  input.setAttribute('autocomplete', 'off');
  input.setAttribute('aria-autocomplete', 'list');
  input.setAttribute('aria-controls', listId);
  input.setAttribute('aria-activedescendant', 'morsels-list-selected');
}
