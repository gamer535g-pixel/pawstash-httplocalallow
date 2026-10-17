import type { Attachment } from 'svelte/attachments';
import { layoutState } from '$lib/state/layoutState.svelte';

// A non-passive wheel listener stalls WebKitGTK scrolling, so Linux attaches it only while Ctrl is held.
// Elsewhere it stays: touchpad pinch arrives as Ctrl+wheel with no key events.
export function ctrlWheel(handler: (event: WheelEvent) => void): Attachment<HTMLElement> {
  return (node) => {
    let attached = false;
    const sync = (held: boolean) => {
      if (held === attached) return;
      attached = held;
      if (held) node.addEventListener('wheel', handler, { passive: false });
      else node.removeEventListener('wheel', handler);
    };

    if (!layoutState.isLinux) {
      sync(true);
      return () => sync(false);
    }

    const onKey = (event: KeyboardEvent) => sync(event.ctrlKey);
    const onBlur = () => sync(false);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('keyup', onKey, true);
    window.addEventListener('blur', onBlur);
    return () => {
      sync(false);
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('keyup', onKey, true);
      window.removeEventListener('blur', onBlur);
    };
  };
}
