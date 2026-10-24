export function collapseSection(
  viewport: HTMLElement | null | undefined,
  section: HTMLElement | null,
  collapsedMaxHeight: number,
  scrollBeforeExpand: number | null,
  collapse: () => void
) {
  if (!viewport || !section) {
    collapse();
    return;
  }

  const rect = section.getBoundingClientRect();
  const isAtEnd = rect.bottom <= viewport.getBoundingClientRect().bottom + 80;

  if (isAtEnd) {
    const heightDiff = rect.height - Math.min(rect.height, collapsedMaxHeight);
    collapse();
    if (heightDiff > 0) viewport.scrollTop = Math.max(0, viewport.scrollTop - heightDiff);
    return;
  }

  collapse();
  if (typeof scrollBeforeExpand === 'number') {
    viewport.scrollTo({ top: scrollBeforeExpand, behavior: 'smooth' });
  } else {
    section.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }
}
