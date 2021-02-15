function h(name, attrs, ...children): HTMLBaseElement {
  const el = document.createElement(name);

  Object.entries(attrs).forEach(([key, value]) => {
    el.setAttribute(key, value);
  });

  children.forEach((child) => {
    if (typeof child === 'string') {
      el.textContent += child;
    } else {
      el.appendChild(child);
    }
  });

  return el;
}

export default {
  h,
};
