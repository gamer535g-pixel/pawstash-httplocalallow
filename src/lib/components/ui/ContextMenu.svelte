<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { contextMenuState } from '$lib/state/contextMenuState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import MenuList from '$lib/components/ui/MenuList.svelte';

  let anchor = $state<HTMLDivElement | null>(null);
  let sheet = $state<BottomSheet>();
</script>

<div bind:this={anchor} class="context-anchor" style:left="{contextMenuState.x}px" style:top="{contextMenuState.y}px"></div>

{#if layoutState.isMobile}
  <BottomSheet
    bind:this={sheet}
    isOpen={contextMenuState.open}
    title={contextMenuState.title}
    closeLabel={i18n.t('common.close')}
    onclose={() => contextMenuState.close()}
  >
    <MenuList items={contextMenuState.items} onrun={() => sheet?.close()} />
  </BottomSheet>
{:else}
  <PopoverMenu bind:open={contextMenuState.open} anchorEl={anchor} align="left" width="auto" menuClass="overflow-menu">
    <div class="context-list">
      <MenuList items={contextMenuState.items} onrun={() => contextMenuState.close()} />
    </div>
  </PopoverMenu>
{/if}

<style>
  .context-anchor {
    position: fixed;
    width: 0;
    height: 0;
    pointer-events: none;
  }

  .context-list {
    min-width: 220px;
    max-width: min(340px, calc(100vw - 32px));
  }
</style>
