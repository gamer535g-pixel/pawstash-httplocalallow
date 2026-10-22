import { selectionState, type SelectionScope } from '$lib/state/selectionState.svelte';

const DRAG_THRESHOLD = 6;
const EDGE_ZONE = 56;
const MAX_SCROLL_STEP = 18;

const OWN_PRESS = 'button:not(.grid-tile-open):not(.grid-tile-select-checkbox), a, input, [role="link"], .tile-toolbar';

function scrollParent(node: HTMLElement): HTMLElement | null {
  for (let el = node.parentElement; el; el = el.parentElement) {
    const { overflowY } = getComputedStyle(el);
    if ((overflowY === 'auto' || overflowY === 'scroll') && el.scrollHeight > el.clientHeight) return el;
  }
  return null;
}

export interface PaintSelectOptions {
  scope: SelectionScope;
  keys: string[];
  items: Map<string, any>;
}

export function paintSelect(node: HTMLElement, options: PaintSelectOptions) {
  let current = options;
  let pointerId: number | null = null;
  let startX = 0;
  let startY = 0;
  let startKey: string | null = null;
  let painting = false;
  let mode: 'select' | 'deselect' = 'select';
  let base = new Set<string>();
  let lastX = 0;
  let lastY = 0;
  let scroller: HTMLElement | null = null;
  let frame = 0;

  function keyAt(x: number, y: number): string | null {
    const tile = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-select-key]');
    return tile && node.contains(tile) ? tile.dataset.selectKey ?? null : null;
  }

  function paintTo(x: number, y: number) {
    const key = keyAt(x, y);
    if (key && startKey) selectionState.paintRange(base, startKey, key, mode, current.keys, current.items);
  }

  function autoScroll() {
    frame = 0;
    if (!painting || !scroller) return;
    const rect = scroller.getBoundingClientRect();
    let step = 0;
    if (lastY < rect.top + EDGE_ZONE) step = -Math.ceil(((rect.top + EDGE_ZONE - lastY) / EDGE_ZONE) * MAX_SCROLL_STEP);
    else if (lastY > rect.bottom - EDGE_ZONE) step = Math.ceil(((lastY - rect.bottom + EDGE_ZONE) / EDGE_ZONE) * MAX_SCROLL_STEP);
    if (step !== 0) {
      scroller.scrollTop += step;
      paintTo(lastX, lastY);
    }
    frame = requestAnimationFrame(autoScroll);
  }

  function suppressClick(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
  }

  function begin() {
    painting = true;
    if (!selectionState.active || selectionState.scope !== current.scope) {
      selectionState.enter(current.scope);
    }
    mode = startKey && selectionState.isSelected(startKey) ? 'deselect' : 'select';
    base = new Set(selectionState.selectedKeys);
    scroller = scrollParent(node);
    document.documentElement.classList.add('is-paint-selecting');
    paintTo(startX, startY);
    frame = requestAnimationFrame(autoScroll);
  }

  function end() {
    if (painting) {
      // The release lands on the tile's open button; that click must not open the post.
      window.addEventListener('click', suppressClick, { capture: true, once: true });
      setTimeout(() => window.removeEventListener('click', suppressClick, { capture: true }), 0);
    }
    painting = false;
    pointerId = null;
    startKey = null;
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    document.documentElement.classList.remove('is-paint-selecting');
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
    window.removeEventListener('pointercancel', onUp);
  }

  function onMove(event: PointerEvent) {
    if (event.pointerId !== pointerId) return;
    lastX = event.clientX;
    lastY = event.clientY;
    if (!painting) {
      if (Math.hypot(lastX - startX, lastY - startY) < DRAG_THRESHOLD) return;
      begin();
    }
    paintTo(lastX, lastY);
  }

  function onUp(event: PointerEvent) {
    if (event.pointerId === pointerId) end();
  }

  function onDown(event: PointerEvent) {
    if (event.pointerType !== 'mouse' || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey) return;
    const target = event.target as HTMLElement;
    if (target.closest(OWN_PRESS)) return;
    const tile = target.closest<HTMLElement>('[data-select-key]');
    if (!tile || !node.contains(tile)) return;
    pointerId = event.pointerId;
    startKey = tile.dataset.selectKey ?? null;
    startX = lastX = event.clientX;
    startY = lastY = event.clientY;
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);
  }

  node.addEventListener('pointerdown', onDown);

  return {
    update(next: PaintSelectOptions) {
      current = next;
    },
    destroy() {
      end();
      node.removeEventListener('pointerdown', onDown);
    }
  };
}
