export function scrollListContainer(targetEl: any, scrollContainer: HTMLElement) {
  const computedStyles = getComputedStyle(scrollContainer);
  if (['scroll', 'auto', 'overlay'].includes(computedStyles.overflowY)) {
    const top = targetEl.offsetTop
      - scrollContainer.offsetTop
      - scrollContainer.clientHeight / 2
      + targetEl.clientHeight / 2;
    scrollContainer.scrollTo({ top });
  } else {
    targetEl.scrollIntoView({
      block: 'center',
    });
  }
}
