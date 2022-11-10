function h(
  name: string,
  attrs: { [attrName: string]: string },
  ...children: (string | HTMLElement)[]
): HTMLElement {
  const el = document.createElement(name);
  
  Object.entries(attrs).forEach(([key, value]) => {
    el.setAttribute(key, value);
  });

  el.append(...children);
  
  return el;
}
  
export default h;
  
export type CreateElement = (
  name: string,
  attrs: { [attrName: string]: string },
  ...children: (string | HTMLElement)[]
) => HTMLElement;
  