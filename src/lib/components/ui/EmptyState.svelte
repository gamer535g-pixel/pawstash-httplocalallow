<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  interface Props {
    icon?: Component;
    title: string;
    description?: string;
    tone?: 'neutral' | 'error';
    children?: Snippet;
  }

  let { icon: Icon, title, description, tone = 'neutral', children }: Props = $props();
</script>

<div class="empty-state" class:is-error={tone === 'error'} role={tone === 'error' ? 'alert' : undefined}>
  {#if Icon}
    <span class="empty-state__icon"><Icon /></span>
  {/if}
  <strong class="empty-state__title">{title}</strong>
  {#if description}
    <span class="empty-state__description">{description}</span>
  {/if}
  {#if children}
    <div class="empty-state__actions">{@render children()}</div>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: max(310px, 50vh);
    padding: 32px 16px;
    text-align: center;
  }

  .empty-state__icon {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    margin-bottom: 12px;
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--text-secondary);
  }

  .empty-state__icon :global(svg) {
    width: 28px;
    height: 28px;
  }

  .empty-state.is-error .empty-state__icon {
    color: var(--status-error);
  }

  .empty-state__title {
    max-width: 480px;
    color: var(--text-primary);
    font-size: var(--type-title-md-size);
    line-height: var(--type-title-md-line);
    font-weight: 500;
    text-wrap: balance;
  }

  .empty-state__description {
    max-width: 420px;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    text-wrap: balance;
    overflow-wrap: anywhere;
  }

  .empty-state__actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 20px;
  }
</style>
