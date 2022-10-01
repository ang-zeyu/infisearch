function h(
  name: string,
  attrs: { [attrName: string]: string },
  ...children: (string | HTMLElement)[]
): HTMLElement {
  const el = document.createElement(name);

  Object.entries(attrs).forEach(([key, value]) => {
    el.setAttribute(key, value);
  });

  children.forEach((child) => {
    if (typeof child === 'string') {
      const span = document.createElement('span');
      span.textContent = child;
      el.appendChild(span);
    } else {
      el.appendChild(child);
    }
  });

  return el;
}

export default h;

export type CreateElement = (
  name: string,
  attrs: { [attrName: string]: string },
  ...children: (string | HTMLElement)[]
) => HTMLElement;

export const LOADING_INDICATOR_ID = 'data-morsels-loading-indicator';

export function createInvisibleLoadingIndicator(): HTMLElement {
  return h('div', { [LOADING_INDICATOR_ID]: 'true' });
}
