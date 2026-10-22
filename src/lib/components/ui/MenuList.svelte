<script lang="ts" module>
  import type { Component } from 'svelte';

  export interface MenuItem {
    label: string;
    icon: Component;
    onclick: () => void;
    detail?: string;
    detailMono?: boolean;
    group?: string;
    disabled?: boolean;
    active?: boolean;
    danger?: boolean;
  }
</script>

<script lang="ts">
  import { ripple } from '$lib/motion';

  interface Props {
    items: MenuItem[];
    onrun: () => void;
  }

  let { items, onrun }: Props = $props();

  function run(item: MenuItem) {
    if (item.disabled) return;
    onrun();
    item.onclick();
  }
</script>

<div class="menu-list" role="menu">
  {#each items as item, index}
    {#if index > 0 && item.group !== items[index - 1].group}
      <div class="floating-divider" role="separator"></div>
    {/if}
    <button
      type="button"
      class="floating-item"
      class:is-active={item.active}
      class:is-danger={item.danger}
      role="menuitem"
      disabled={item.disabled}
      use:ripple
      onclick={() => run(item)}
    >
      <item.icon />
      <span class="floating-item-label">{item.label}</span>
      {#if item.detail}
        <span class="floating-item-detail" class:is-mono={item.detailMono}>{item.detail}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .menu-list {
    display: flex;
    flex-direction: column;
  }
</style>
