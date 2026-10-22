<script lang="ts">
  import type { Snippet } from 'svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { i18n } from '$lib/i18n';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconSelectAll from '~icons/fluent/select-all-on-24-regular';
  import IconSelectAllOff from '~icons/fluent/select-all-off-24-regular';

  interface Props {
    total: number;
    onSelectAll?: () => void;
    actions?: Snippet;
  }

  let { total, onSelectAll, actions }: Props = $props();

  let count = $derived(selectionState.count);
  let allSelected = $derived(total > 0 && count >= total);
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape' && selectionState.active && !event.defaultPrevented) selectionState.exit();
  }}
/>

<Button
  variant="ghost"
  class="btn-icon bar-back"
  tooltip={layoutState.hasKeyboard ? `${i18n.t('selection.cancel')} (Esc)` : i18n.t('selection.cancel')}
  onclick={() => selectionState.exit()}
>
  <IconDismiss />
</Button>

<h1 class="selection-count" aria-live="polite">
  {#if layoutState.isMobile}
    {i18n.number(count)}
  {:else}
    {count === 0 ? i18n.t('selection.none_selected') : `${i18n.number(count)} ${i18n.t('selection.items_count')}`}
  {/if}
</h1>

{#if onSelectAll && total > 0}
  <Button
    variant="ghost"
    class="btn-icon"
    tooltip={i18n.t(allSelected ? 'selection.deselect_all' : 'selection.select_all')}
    aria-pressed={allSelected}
    onclick={() => (allSelected ? selectionState.clear() : onSelectAll())}
  >
    {#if allSelected}<IconSelectAllOff />{:else}<IconSelectAll />{/if}
  </Button>
{/if}

<div class="selection-spacer"></div>

{#if actions}
  <div class="selection-actions" class:is-disabled={count === 0}>
    {@render actions()}
  </div>
{/if}

<style>
  .selection-count {
    margin: 0 8px 0 4px;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: var(--type-title-lg-size);
    line-height: var(--type-title-lg-line);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .selection-spacer {
    flex: 1 1 0;
    min-width: 0;
  }

  .selection-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .selection-actions.is-disabled {
    opacity: 0.38;
    pointer-events: none;
  }
</style>
