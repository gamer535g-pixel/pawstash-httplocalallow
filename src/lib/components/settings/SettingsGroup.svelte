<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label?: string;
    children: Snippet;
  }

  let { label, children }: Props = $props();
</script>

<section class="settings-group" aria-label={label}>
  {#if label}
    <h2 class="settings-group__label">{label}</h2>
  {/if}
  <div class="settings-group__tiles">
    {@render children()}
  </div>
</section>

<style>
  .settings-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .settings-group__label {
    margin: 0;
    padding-inline: 16px;
    color: var(--text-secondary);
    font-size: var(--type-title-sm-size);
    line-height: var(--type-title-sm-line);
    font-weight: 500;
  }

  .settings-group__tiles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .settings-group__tiles > :global(:first-child) {
    border-top-left-radius: var(--radius-xl);
    border-top-right-radius: var(--radius-xl);
  }

  .settings-group__tiles > :global(:last-child) {
    border-bottom-left-radius: var(--radius-xl);
    border-bottom-right-radius: var(--radius-xl);
  }
</style>
