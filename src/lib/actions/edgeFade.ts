export function edgeFade(node: HTMLElement) {
  const update = () => {
    const end = node.scrollWidth - node.clientWidth;
    node.toggleAttribute('data-fade-start', node.scrollLeft > 1);
    node.toggleAttribute('data-fade-end', node.scrollLeft < end - 1);
  };
  const observer = new ResizeObserver(update);
  observer.observe(node);
  for (const child of node.children) observer.observe(child);
  node.classList.add('edge-fade');
  node.addEventListener('scroll', update, { passive: true });
  update();
  return {
    destroy() {
      node.removeEventListener('scroll', update);
      observer.disconnect();
    }
  };
}
