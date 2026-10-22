import type { MenuItem } from '$lib/components/ui/MenuList.svelte';

class ContextMenuState {
  open = $state(false);
  title = $state('');
  items = $state<MenuItem[]>([]);
  x = $state(0);
  y = $state(0);

  show(event: MouseEvent, title: string, items: MenuItem[]) {
    event.preventDefault();
    this.x = event.clientX;
    this.y = event.clientY;
    this.title = title;
    this.items = items;
    this.open = true;
  }

  close() {
    this.open = false;
  }
}

export const contextMenuState = new ContextMenuState();
