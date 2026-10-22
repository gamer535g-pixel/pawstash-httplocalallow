<script lang="ts">
  import type { Snippet } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import SearchBar from '$lib/components/ui/SearchBar.svelte';

  interface Props {
    searchOpen?: boolean;
    searchQuery?: string;
    searchPlaceholder?: string;
    showSearchButton?: boolean;
    pinOpen?: boolean;
    onsearchtoggle?: (open: boolean) => void;
    children?: Snippet;
    class?: string;
  }

  let {
    searchOpen = $bindable(false),
    searchQuery = $bindable(''),
    searchPlaceholder,
    showSearchButton = true,
    pinOpen = false,
    onsearchtoggle,
    children,
    class: extraClass = ''
  }: Props = $props();

  const PINNED_MIN_WIDTH = 1100;
  const isMac = layoutState.isMacOS;
  let searchBar = $state<ReturnType<typeof SearchBar> | null>(null);
  let pinned = $derived(pinOpen && !layoutState.isMobile && layoutState.screenWidth >= PINNED_MIN_WIDTH);

  function handleShortcut(event: KeyboardEvent) {
    if (!showSearchButton || event.key.toLowerCase() !== 'k' || !(isMac ? event.metaKey : event.ctrlKey)) return;
    event.preventDefault();
    if (pinned) searchBar?.focusInput();
    else void searchBar?.openSearch();
  }

  // Narrowing the window with a query typed must not hide the field that explains the filtered results.
  $effect(() => {
    if (!pinned && searchQuery && !searchOpen) searchOpen = true;
  });
</script>

<svelte:window onkeydown={handleShortcut} />

<div
  class="header-actions-root {extraClass}"
  class:is-mobile={layoutState.isMobile}
  class:search-active={searchOpen}
>
  {#if showSearchButton}
    <SearchBar
      bind:this={searchBar}
      shortcutHint={layoutState.hasKeyboard ? (isMac ? '⌘K' : 'Ctrl K') : undefined}
      bind:value={searchQuery}
      bind:open={searchOpen}
      expandable={!pinned}
      placeholder={searchPlaceholder}
      onclose={() => onsearchtoggle?.(false)}
    />
  {/if}

  {#if !layoutState.isMobile || !searchOpen}
    <div class="action-buttons-cluster">
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</div>

<style>
  .header-actions-root {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .header-actions-root.is-mobile.search-active {
    width: 100%;
    flex: 1;
    display: flex;
    align-items: center;
    background: transparent;
  }

  .action-buttons-cluster {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
